extern crate clap;
extern crate yaml_rust;
extern crate regex;
extern crate chrono;

extern crate libc;
extern crate pentry;

use clap::ArgMatches;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Debug)]
enum Shell {
    Fish,
    Bash
}

#[derive(Debug)]
pub struct LoxArgs {
    show_timestamp: bool,
    show_index: bool,
}

#[derive(Debug)]
struct Command {
    time: i64,
    cmd: String,
}

#[derive(Debug)]
struct ShellHistory {
    history: Vec<Command>,
    shell: Shell
}

pub fn process_args(matches: ArgMatches) -> LoxArgs {
    LoxArgs {
        show_timestamp: match matches.occurrences_of("t") {
            1 => true,
            _ => false,
        },
        show_index: match matches.occurrences_of("n") {
            1 => true,
            _ => false,
        },
    }
}

fn clean(line: &str) -> String {
    if line.matches(":").count() > 1 {
        let newline = String::from(line);
        use self::regex::Regex;
        let re = Regex::new(r"^\- \w+: (.*)$").unwrap();

        if re.is_match(newline.as_str()) {
            let cap = re.captures(newline.as_str()).unwrap();
            let out = format!("- cmd: \"{}\"", &cap[1]);
            return out;
        } else {
            panic!("Bad match!");
        }
    } else {
        return line.to_string();
    }
}

fn get_parent_shell() -> String {
    let pid: i32;
    unsafe {
        pid = libc::getppid() as i32;
    }

    if let Ok(ps) = pentry::find(pid) {
        let prog_option = ps.path().unwrap().split("/").collect::<Vec<&str>>();

        let program_name = match prog_option.last() {
            Some(&v) => return v.to_owned(),
            _ => panic!("Unable to get shell name"),
        };
    } else {
        panic!("Unable to find shell PID")
    }
}

fn bash_history() -> ShellHistory {
    let home_directory = env!("HOME");
    let bash_history_path = home_directory.to_owned() + "/.bash_history";

    let mut file = match File::open(bash_history_path.to_string()) {
        Ok(v) => v,
        Err(e) => panic!("Fish file not found"),
    };

    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(v) => (),
        Err(e) => panic!("Unable to read file"),
    };

    return ShellHistory {
        shell: Shell::Bash,
        history: contents
            .as_str()
            .split("\n")
            .collect::<Vec<&str>>()
            .into_iter()
            .map(|x| {
                Command {
                    cmd : String::from(x),
                    time : -1
                }
            })
            .collect()
    }
}

fn fish_history() -> ShellHistory {
    use self::yaml_rust::{YamlLoader, YamlEmitter, Yaml};

    let home_directory = env!("HOME");
    let fish_history_path = home_directory.to_owned() + "/.local/share/fish/fish_history";
    let mut file = match File::open(fish_history_path.to_string()) {
        Ok(v) => v,
        Err(e) => panic!("Fish file not found"),
    };

    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(v) => (),
        Err(e) => panic!("Unable to read file"),
    };

    let mut sanitized: String = contents
        .as_str()
        .split("\n")
        .collect::<Vec<&str>>()
        .into_iter()
        .map(|x| clean(x).replace("\"", "\\\""))
        .collect::<Vec<String>>()
        .join("\n");

    let parsed_history = match YamlLoader::load_from_str(sanitized.as_str()) {
        Ok(v) => v,
        Err(e) => panic!("Unable to parse fish history"),
    };

    let mut out: Vec<Command> = match parsed_history[0].as_vec() {
        Some(col) => {
            col.into_iter()
                .map(|item| {
                         Command {
                             time: item["when"].as_i64().unwrap(),
                             cmd: String::from(item["cmd"].as_str().unwrap()),
                         }
                     })
                .collect()
        }
        None => panic!("Unable to parse fish history"),
    };

    return ShellHistory {
        history: out,
        shell: Shell::Fish
    };
}

pub fn lox_main(matches: ArgMatches) {
    use self::chrono::prelude::*;

    let args: LoxArgs = process_args(matches);
    let shell_history: ShellHistory = match get_parent_shell().as_ref() {
        "fish" => fish_history(),
        "bash" => bash_history(),
        _ => panic!(format!("Unsupported shell: {}",  get_parent_shell()))
    };

    let mut idx = 0;

    for item in shell_history.history {
        let timestamp = match shell_history.shell {
          Shell::Fish => match args.show_timestamp {
              true => format!("{}\t", NaiveDateTime::from_timestamp(item.time, 0)),
              false => String::from(""),
          },
          _ => String::from("")
        };

        let index = match args.show_index {
            true => format!("{}\t", idx),
            false => String::from(""),
        };

        println!("{}{}{}", index, timestamp, item.cmd);
        idx += 1;
    }
}
