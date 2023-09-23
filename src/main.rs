extern crate core;

use arboard::Clipboard;
use clap::{Parser, Subcommand};
use prettytable::{color, Attr, Cell, Row, Table};
use regex::Regex;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::string::String;
use std::{env, io};
use std::fmt::{Display, Formatter};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file path
    #[clap(short, long, value_parser, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Subcommand to use
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TunnelModeError {
    message: String,
}

impl TunnelModeError {
    pub fn new(message: &str) -> Self {
        TunnelModeError {
            message: message.to_string()
        }
    }
}

impl Error for TunnelModeError {}

impl Display for TunnelModeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tunnel Mode Error: {}", self.message)
    }
}


#[derive(Subcommand)]
enum Commands {
    /// Shows the current configuration
    Show {
        /// Optional filter for the connections
        filter: Option<String>
    },
    /// launches the ssh command for the selected index of the table or the specified connection name in the table
    Use {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Additional args to use in the command
        #[clap(short, long, value_parser, value_name = "args")]
        args: Option<Vec<String>>,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("ssh"))]
        command: String,
    },
    /// exports the ssh command for the selected index of the table or the specified connection name in the table to the clipboard
    Export {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Additional args to use in the command
        #[clap(short, long, value_parser, value_name = "args")]
        args: Option<Vec<String>>,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("ssh"))]
        command: String,
    },
    /// launches the scp command for the selected index of the table or the specified connection mane in the table, use "con:"<path> to be replaced with the connection to the selected ssh server
    Copy {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Path to copy the file from, use "con:" at the beginning to indicate that this path is on the remote server.
        #[clap(value_parser, value_name = "From")]
        from: String,

        /// Path to copy the file to, use "con:" at the beginning to indicate that this path is on the remote server.
        #[clap(value_parser, value_name = "To")]
        to: String,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("scp"))]
        command: String,
    },

    /// Adds a new entry to the ssh config file.
    Add {
        /// Specifies the host name to store the new entry
        #[clap(value_parser, value_name = "Host")]
        host: String,

        /// Specifies the real host name to log into
        #[clap(value_parser, value_name = "HostName")]
        host_name: String,

        /// Specifies the user to log in as
        #[clap(value_parser, value_name = "User")]
        user: String,

        /// Specifies the port number to connect on the remote host.
        #[clap(
        short,
        long,
        value_parser,
        value_name = "port",
        default_value_t = 22u32
        )]
        port: u32,

        /// Specifies a file from which the user's authentication identity is read
        #[clap(short, long, value_parser, value_name = "IdentityFile")]
        identity_file: Option<String>,

        /// Specifies that ssh should only use the identity keys configured in the ssh_config files, even if ssh-agent offers more identities.
        #[clap(
        short('y'),
        long,
        value_parser,
        value_name = "IdentitiesOnly",
        default_value_t = false
        )]
        identities_only: bool,
    },
    /// Deletes an entry from the ssh config file
    Delete {
        /// Index of the selected entry to delete
        #[clap(value_parser, value_name = "Selection")]
        selection: String,
    },
    /// makes a ssh tunnel for the selected index of the table or the specified connection name in the table
    Tunnel {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("ssh"))]
        command: String,

        /// tunnel mode
        #[clap(subcommand)]
        mode: Option<TunnelMode>,

        /// Additional args to use in the command
        #[clap(short, long, value_parser, value_name = "args")]
        args: Option<Vec<String>>,
    },
}

#[derive(Subcommand)]
enum TunnelMode {
    Local {
        /// Local port to use as one of the sides of the tunnel
        #[clap(value_parser, value_name = "LocalPort")]
        local_port: u16,

        /// Remote host to forward traffic from the tunnel
        #[clap(value_parser, value_name = "RemoteHost", default_value_t = String::from("127.0.0.1"))]
        remote_host: String,

        /// Remote port to forward traffic from the tunnel
        #[clap(value_parser, value_name = "RemotePort", default_value = "80")]
        remote_port: u16,
    },
    Remote {
        /// Local port to forward traffic from the tunnel
        #[clap(value_parser, value_name = "LocalPort")]
        local_port: u16,

        /// Local host to forward traffic from the tunnel
        #[clap(value_parser, value_name = "LocalHost", default_value_t = String::from("127.0.0.1"))]
        local_host: String,

        /// Remote port to forward traffic to the tunnel
        #[clap(value_parser, value_name = "RemotePort", default_value = "80")]
        remote_port: u16,
    },
    Dynamic {
        /// Local port to use as one of the sides of the tunnel
        #[clap(value_parser, value_name = "LocalPort")]
        local_port: u16,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    #[allow(deprecated)]
        let mut buf = env::home_dir().unwrap();
    buf.push(Path::new(".ssh/config"));
    let config_file = match cli.config {
        Some(path) => path,
        None => buf,
    };

    let config_file = config_file.as_path();
    let config_file = config_file.canonicalize().unwrap();
    let config_file = config_file.as_path();

    if cli.command.is_some() {
        let data: Result<Vec<Vec<String>>, Box<dyn Error>> = read_ssh_config_file(config_file);
        return match &cli.command {
            Some(Commands::Show {
                     filter
                 }) => {
                if !(config_file.exists() && config_file.is_file()) {
                    panic!("couldnt open {:#?}", config_file.as_os_str())
                }
                let filter = filter.clone().map(|filter_str| format!(".*{filter_str}.*")).unwrap_or(String::from(r".*"));
                let filter = Regex::new(filter.as_str()).unwrap();
                data.map(|data| {
                    let mut header = true;
                    data.iter().filter(|row| {
                        let old_header = header;
                        if header {
                            header = false;
                        }
                        row.iter().any(|cell| old_header || filter.is_match(cell))
                    })
                        .map(|row| row.to_owned())
                        .collect::<Vec<Vec<String>>>()
                })
                    .map(|data| {
                        let mut table = Table::new();
                        let mut header = true;
                        for row in data {
                            table.add_row(Row::new(
                                row.iter()
                                    .map(|cell| {
                                        let table_cell = Cell::new(cell.as_str());
                                        if header {
                                            table_cell
                                                .with_style(Attr::Bold)
                                                .with_style(Attr::ForegroundColor(color::GREEN))
                                        } else {
                                            table_cell.with_style(Attr::ForegroundColor(color::CYAN))
                                        }
                                    })
                                    .collect::<Vec<Cell>>(),
                            ));
                            if header {
                                header = false;
                            }
                        }
                        // let table = Table::from(data.iter());
                        table.printstd();
                    })
            }
            Some(Commands::Use {
                     selection,
                     args,
                     command,
                 }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut command = Command::new(command);
                let command = command
                    .arg(connection_name)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
                match args {
                    None => {}
                    Some(args) => {
                        command.args(args.iter());
                    }
                };
                command.spawn().unwrap().wait().unwrap();
            }),
            Some(Commands::Export {
                     selection,
                     args,
                     command,
                 }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut clipboard = Clipboard::new().unwrap();
                let args_str = match args {
                    Some(args) => args.join(" "),
                    None => String::new(),
                };
                clipboard
                    .set_text(format!("{} {} {}", command, connection_name, args_str))
                    .unwrap();
            }),
            Some(Commands::Copy {
                     selection,
                     from,
                     to,
                     command,
                 }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut command = Command::new(command);
                let from = from.replace("con:", format!("{}:", connection_name).as_str());
                let to = to.replace("con:", format!("{}:", connection_name).as_str());
                let command = command
                    .arg(from)
                    .arg(to)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
                command.spawn().unwrap().wait().unwrap();
            }),
            Some(Commands::Add {
                     host,
                     host_name,
                     user,
                     port,
                     identity_file,
                     identities_only,
                 }) => {
                if !(config_file.exists() && config_file.is_file()) {
                    panic!("couldnt find {:#?}", config_file.as_os_str())
                }
                let host_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(config_file);
                match host_file {
                    Ok(mut host_file) => add_entry(
                        &mut host_file,
                        host,
                        host_name,
                        user,
                        port,
                        identity_file,
                        identities_only,
                    ),
                    Err(e) => Err(Box::new(e)),
                }
            }
            Some(Commands::Delete { selection }) => selection
                .trim()
                .parse::<usize>()
                .map(|selected_index| selected_index as i32)
                .map(|mut selected_index| {
                    let read_file =
                        OpenOptions::new()
                            .read(true)
                            .open(config_file)
                            .map(|host_file| {
                                let lines = BufReader::new(host_file).lines();
                                let mut copy = true;
                                let mut new_host_contents = String::new();
                                use std::fmt::Write;
                                let mut selected_host = None;
                                for line in lines.flatten() {
                                    if line.starts_with("Host ") {
                                        if selected_index != 0 {
                                            copy = true;
                                            writeln!(new_host_contents).expect("");
                                        } else {
                                            selected_host = Some(
                                                line.replace("Host ", "").trim().to_string(),
                                            );
                                            copy = false;
                                        }
                                        selected_index -= 1;
                                    }
                                    if copy {
                                        writeln!(new_host_contents, "{line}").expect("");
                                    }
                                }
                                let mut confirmation = false;
                                if let Some(selected_host) = selected_host {
                                    println!(
                                        "The host \"{selected_host}\" will be deleted, are you sure?"
                                    );
                                    println!("Type \"yes\" to confirm");
                                    let stdin = io::stdin();
                                    let mut response = String::new();
                                    stdin.read_line(&mut response).unwrap();
                                    confirmation = response.trim() == "yes";
                                }
                                (new_host_contents, confirmation)
                            });
                    if let Ok((new_host_contents, confirmation)) = read_file {
                        if confirmation {
                            OpenOptions::new()
                                .write(true)
                                .open(config_file)
                                .map(|host_file| {
                                    host_file.set_len(0).unwrap();
                                    BufWriter::new(host_file)
                                        .write_all(new_host_contents.as_bytes())
                                })
                                .unwrap()
                                .unwrap();
                        }
                    }
                })
                .map_err(|e| Box::new(e) as Box<dyn Error>),
            Some(Commands::Tunnel {
                     selection,
                     command,
                     mode,
                     args
                 }) => {
                match mode {
                    None => {
                        return Err(Box::new(TunnelModeError::new("no tunnel mode selected")));
                    }
                    Some(tunnel_mode) => {
                        data.map(|data| {
                            let connection_name = get_connection_name(data, selection);
                            let mut command = Command::new(command);
                            match tunnel_mode {
                                TunnelMode::Local {
                                    local_port,
                                    remote_host,
                                    remote_port
                                } => {
                                    command.arg("-L")
                                        .arg(local_port.to_string())
                                        .arg(":")
                                        .arg(remote_host)
                                        .arg(":")
                                        .arg(remote_port.to_string())
                                }
                                TunnelMode::Remote {
                                    local_port,
                                    local_host ,
                                    remote_port
                                } => {
                                    command.arg("-R")
                                        .arg(remote_port.to_string())
                                        .arg(":")
                                        .arg(local_host)
                                        .arg(":")
                                        .arg(local_port.to_string())
                                }
                                TunnelMode::Dynamic {
                                    local_port
                                } => {
                                    command.arg("-D")
                                        .arg(local_port.to_string())
                                }
                            };
                            let command = command
                                .arg(connection_name)
                                .stdin(Stdio::inherit())
                                .stdout(Stdio::inherit())
                                .stderr(Stdio::inherit());
                            match args {
                                None => {}
                                Some(args) => {
                                    command.args(args.iter());
                                }
                            };
                            command.spawn().unwrap().wait().unwrap();
                        })
                    }
                }.expect("Error creating tunnel");
                return Ok(());
            }
            None => {
                return Ok(());
            }
        };
    }
    Ok(())
}

fn add_entry(
    host_file: &mut File,
    host: &String,
    host_name: &String,
    user: &String,
    port: &u32,
    identity_file: &Option<String>,
    identities_only: &bool,
) -> Result<(), Box<dyn Error>> {
    writeln!(host_file)?;
    writeln!(
        host_file,
        "Host {host}
    HostName {host_name}
    user {user}
    port {port}"
    )?;
    if let Some(identity_file_path) = identity_file {
        writeln!(host_file, "    IdentityFile {identity_file_path}")?;
    }
    if *identities_only {
        writeln!(host_file, "    IdentityFilesOnly yes")?;
    }
    Ok(())
}

fn get_connection_name(data: Vec<Vec<String>>, index: &String) -> String {
    match index.trim().parse::<usize>() {
        Ok(index) => {
            if index > data.len() {
                panic!(
                    "incorrect index ({}), max index = {}",
                    index,
                    data.len() - 1
                )
            }
            String::from(&data[index + 1][1])
        }
        Err(_) => {
            if data.iter().filter(|&row| row[1].eq(index)).count() == 0 {
                panic!("no connection in the list with the name {}", index);
            }
            String::from(index)
        }
    }
}

fn read_ssh_config_file(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .map(|mut file| {
            let mut data = String::new();
            file.read_to_string(&mut data).expect("Error reading file");
            data
        })
        .map(|data| {
            let mut row = 0;
            let mut first = true;
            let mut data_matrix = vec![];
            for line in data.lines() {
                let line = String::from(line);
                if line.trim().starts_with("Host ") {
                    if first {
                        first = false;
                    } else {
                        row += 1;
                    }
                    let mut data_row = vec![String::new(); 4];
                    data_row[0] = format!("{}", row);
                    data_row[1] = line.trim().replace("Host ", "");
                    data_matrix.push(data_row);
                } else if line.trim().starts_with("HostName ") {
                    let data_row = data_matrix.get_mut(row).unwrap();
                    data_row[2] = line.trim().replace("HostName ", "");
                } else if line.trim().starts_with("user ") {
                    let data_row = data_matrix.get_mut(row).unwrap();
                    data_row[3] = line.trim().replace("user ", "");
                }
            }
            data_matrix
        })
        .map(|data_matrix| {
            let mut data_with_title = vec![vec![
                String::from("Index"),
                String::from("HostName"),
                String::from("Host"),
                String::from("User"),
            ]];
            data_matrix.iter().for_each(|row| {
                let row = row.to_vec();
                data_with_title.push(row)
            });
            data_with_title
        })
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}
