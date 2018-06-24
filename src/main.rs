extern crate clap;
extern crate fs_extra;
extern crate pulldown_cmark;

use std::path::{Path, PathBuf};

fn parse_args() -> Result<(PathBuf, Option<PathBuf>), String> {
    use clap::{Arg, App};

    let matches = App::new("mdtest")
        .version("0.1")
        .arg(Arg::with_name("testdir")
             .long("testdir")
             .takes_value(true)
             .value_name("DIR")
             .help("Sets test directory"))
        .arg(Arg::with_name("file")
             .help("Markdown file")
             .required(true))
        .get_matches();

    let file = Path::new(matches.value_of("file").unwrap())
        .canonicalize()
        .map_err(|err| err.to_string())?;

    if !file.is_file() {
        return Err(format!("{} is not a file", file.to_str().unwrap()));
    }

    let testdir = match matches.value_of("testdir") {
        Some(testdir) => {
            let testdir = Path::new(testdir).to_path_buf();
            if testdir.exists() {
                return Err(format!("{} exists", testdir.to_str().unwrap()));
            }
            Some(testdir)
        }
        _ => None,
    };

    Ok((file, testdir))
}

fn prepare_env(file: PathBuf, testdir: Option<PathBuf>) -> Result<String, String> {
    use std::fs;
    use std::env;

    let file_dir = file.parent().unwrap();
    let file = fs::read_to_string(&file).map_err(|err| err.to_string())?;

    let () = match testdir {
        Some(out_dir) => {
            let mut test_env = Vec::new();

            for entry in file_dir.read_dir().unwrap() {
                test_env.push(entry.unwrap().path());
            }

            let () = fs::create_dir_all(&out_dir).unwrap();
            {
                use fs_extra::*;

                let mut opts = dir::CopyOptions::new();
                opts.copy_inside = true;
                let _ = copy_items(&test_env, &out_dir, &opts).unwrap();
            }

            env::set_current_dir(out_dir)
        },
        None => env::set_current_dir(file_dir),
    }.unwrap();

    Ok(file)
}

enum CodeTy {
    Shell,
    FileExist,
}

struct CodeBlock {
    ty: CodeTy,
    ignore: bool,
    code: String,
}

impl CodeBlock {
    fn new(ty: CodeTy) -> CodeBlock {
        CodeBlock {
            ty: ty,
            ignore: false,
            code: String::new(),
        }
    }

    fn run(&self) -> Result<(), String> {
        use std::process::*;
        use std::io::prelude::*;

        if self.ignore {
            return Ok(());
        }

        match self.ty {
            CodeTy::Shell => {
                let mut cmd = Command::new("sh")
                    .arg("-s")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|err| err.to_string())?;

                {
                    let mut code = String::new();
                    code.push_str("set -euo pipefail\n");
                    code.push_str(&self.code);
                    let () = cmd.stdin.as_mut().unwrap()
                        .write_all(code.as_bytes())
                        .map_err(|err| err.to_string())?;
                }

                let out = cmd.wait_with_output().map_err(|err| err.to_string())?;
                if !out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(format!(
                        "shell:\n\ncode {{\n{}\n}}\n\nstdout {{\n{}\n}}\n\nstderr {{\n{}\n}}",
                        self.code, stdout, stderr
                    ));
                }
            },
            CodeTy::FileExist => {
                for file in self.code.lines() {
                    if !Path::new(file).exists() {
                        return Err(format!("file-exist: {} does not exist", file));
                    }
                }
            },
        }

        Ok(())
    }

    fn append(&mut self, text: &str) {
        self.code.push_str(text)
    }
}

fn parse_code_info(info: &str) -> Option<CodeBlock> {
    let mut tokens = info.split(",");

    let mut code = if let Some(lang) = tokens.next() {
        match lang.trim() {
            "sh" => CodeBlock::new(CodeTy::Shell),
            "file-exist" => CodeBlock::new(CodeTy::FileExist),
            _ => return None,
        }
    } else {
        return None;
    };

    for tok in tokens {
        match tok.trim() {
            "ignore" => code.ignore = true,
            _ => return None,
        }
    }

    Some(code)
}

fn run_tests(file: String) -> Result<(), String> {
    use pulldown_cmark::*;

    let mut code = None;

    for event in Parser::new(&file) {
        match event {
            Event::Start(Tag::CodeBlock(info)) => {
                code = parse_code_info(&info);
            }
            Event::End(Tag::CodeBlock(_)) => {
                if let Some(code) = code {
                    let () = code.run()?;
                }
                code = None;
            }
            Event::Text(text) => {
                if let Some(ref mut code) = code {
                    code.append(&text);
                }
            }
            _ => {},
        }
    }

    Ok(())
}

fn main() {
    use std::process::{exit};

    let (file, testdir) = match parse_args() {
        Ok(ret) => ret,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };

    let file = match prepare_env(file, testdir) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };

    match run_tests(file) {
        Ok(_) => println!("All test passed"),
        Err(err) => {
            eprintln!("Error:\n\n{}", err);
            exit(1);
        },
    }
}
