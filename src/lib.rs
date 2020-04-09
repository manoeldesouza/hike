//! A bare-bones HTTP server library with dynamic page capabilties

use std::borrow;
use std::fs;
use std::io::prelude::*;
use std::net;
use std::path;
use std::thread;

type Function = fn() -> String;


/// Instanciates the HTTP server with specific details about address and TCP port.
/// Details about web server root directory (default: local directory), debug mode
/// (default: false) or standard page (default: index.html) can be adjusted after
/// instanciated. Dynamic page functionality is implemented via DynamicPage and
/// Anchor objects
#[derive(Clone)]
pub struct Server {
    ip_address: String,
    tcp_port:   u32,
    debug:      bool,
    root_dir:   path::PathBuf,
    std_page:   String,
    dynamic_pages: Vec<DynamicPage>,
}

/// Each instance of DynamicPage is associated with an unique URL, and can define multiple anchors.
/// This way, multiple portions of the dynamic page can be served by the declared function.
#[derive(Clone)]
pub struct DynamicPage {
    pub url:     String,
    pub anchors: Vec<Anchor>,
}

/// Anchor defines a string from the html page (the marker), which will trigger the execution of a
/// rust function declared by the library user. The function must return a String. The basic rule of
/// execution is that the marker will be replaced by the String resulting of the function execution.
#[derive(Clone)]
#[derive(Debug)]
pub struct Anchor {
    pub marker:   String,
    pub function: Function,
}


impl Server {

    /// Creates an instance of the Serves with general defaults.
    pub fn new(ip_address: String, tcp_port: u32) -> Server {

        Server {
            ip_address: ip_address,
            tcp_port:   tcp_port,
            root_dir:   path::PathBuf::from("."),
            debug:      false,
            std_page:   String::from("index.html"),
            dynamic_pages: Vec::new(),
        }
    }

    /// Sets debugging (lists the URL requested, the related page and HTTP result) on and off.
    pub fn debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Sets a root directory for files to be served. The path presented must be a valid directory.
    /// If the directory does not exist or if it is not a directory, an Err result will be returned.
    pub fn root_dir(&mut self, root_dir: path::PathBuf) -> Result<(), String> {

        if fs::metadata(&root_dir).is_ok() {
            if fs::metadata(&root_dir).unwrap().is_dir() {
                self.root_dir = root_dir;
                Ok(())

            } else {
                Err("Not a directory. Not applied.".to_string())
            }

        } else {
            Err("Does not exists. Not applied.".to_string())
        }
    }

    /// Changes the default page file name (default is index.html)
    pub fn std_page(&mut self, std_page: String) {
        self.std_page = std_page;
    }

    /// Defines dynamic pages to be served by Hike using DynamicPage and Anchor structs.
    pub fn insert_dynamic_page(&mut self, dynamic_page: DynamicPage) {
        self.dynamic_pages.push(dynamic_page);
    }

    /// Enables the Server instance to start serving static and dynamic pages according to the
    /// parameters set.
    pub fn run(&self) {

        let address = format!("{}:{}", self.ip_address, self.tcp_port);
        let listener = net::TcpListener::bind(address).expect("Failure to bind");

        eprintln!("Serving files in current directory via HTTP using port: {}",
            self.tcp_port);

        for stream in listener.incoming() {
            let stream = stream.expect("Failure to read stream");
            let server = self.clone();
            thread::spawn(move || { Server::handle_connection(stream, &server) });
        }
    }

    fn handle_connection(mut stream: net::TcpStream, server: &Server) {

        let mut buffer = [0; 512];

        let request_content = {
            stream.read(&mut buffer).unwrap();
            String::from_utf8_lossy(&buffer[..]).to_string()
        };

        let url = match request_content.split_whitespace().nth(1) {
            Some(url) => url.to_string(),
            None      => return,
        };

        let path = Server::get_path(&url, &server.std_page, &server.root_dir);

        let (http_result, mut file_contents) = match fs::read(&path) {
            Ok(file) => ("200 OK",        file       ),
            Err(_)   => ("404 Not Found", Vec::new() ),
        };

        if server.debug { eprintln!(" {}: {} = {} => {}",
            stream.peer_addr().unwrap(), url, path, http_result);
        }

        match server.dynamic_pages.iter().filter(|x| x.url == url)
                                  .collect::<Vec<&DynamicPage>>().get(0) {
            None => (),
            Some(dynamic_page) => {
                let mut string_file = borrow::Cow::from(String::from_utf8_lossy(&file_contents));
                for anchor in &dynamic_page.anchors {
                if server.debug { eprintln!(" {:?}", anchor); }
                    if string_file.contains(&anchor.marker) {
                        string_file = string_file.replace(&anchor.marker, &(anchor.function)())
                                                 .into();
                    }
                }
                file_contents = string_file.as_bytes().to_vec();
            },
        }

        let response = [
            format!("HTTP/1.1 {}\r\n\r\n", http_result).as_bytes().to_vec(),
            file_contents
        ].concat();

        stream.write(&response).expect("Failure sending response");
        stream.flush().expect("Failure flushing response");
        stream.shutdown(net::Shutdown::Both).expect("shutdown call failed");
    }

    fn get_path(url: &String, std_page: &String, root_dir: &path::PathBuf) -> String {

        let root_dir = root_dir.to_str().unwrap();

        if url.chars().last().unwrap() == '/' {
            format!("{}{}{}", root_dir, url, std_page)

        } else if fs::metadata(format!("{}{}", root_dir, url)).is_ok() &&
                  fs::metadata(format!("{}{}", root_dir, url)).unwrap().is_dir() {
            format!("{}{}/{}", root_dir, url, std_page)

        } else {
            format!("{}{}", root_dir, url)
        }
    }
}



#[cfg(test)]
mod tests {

    use crate::*;
    use std::process;

    #[test]
    fn dynamic_server() {

        let mut server = crate::Server::new("127.0.0.1".to_string(), 8080);
        server.debug(true);

        let anchor1 = crate::Anchor {
            marker: "<!-- [ls] -->".to_string(),
            function: ls_command,
        };

        let dynamic_page1 = crate::DynamicPage {
            url: "/".to_string(),
            anchors: vec![anchor1],
        };

        let anchor2 = crate::Anchor {
            marker: "<!-- [uptime] -->".to_string(),
            function: uptime_command,
        };

        let dynamic_page2 = crate::DynamicPage {
            url: "/dynamic.html".to_string(),
            anchors: vec![anchor2],
        };

        server.insert_dynamic_page(dynamic_page1);
        server.insert_dynamic_page(dynamic_page2);

        match server.root_dir(path::PathBuf::from("example_dynamic")) {
            Ok(_) => (),
            Err(_) => ()}

        server.run();
    }

    #[test]
    fn static_server() {

        let mut server = crate::Server::new("127.0.0.1".to_string(), 8080);
        server.debug(true);
        match server.root_dir(path::PathBuf::from("example_static")) {
            Ok(_) => (),
            Err(_) => ()
        }
        server.run();
    }

    /// This is an example function. Any "void" Rust function returning an String is valid.
    fn uptime_command() -> String {

        let output = process::Command::new("sh")
            .arg("-c")
            .arg("uptime")
            .output()
            .expect("failed to execute process");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// This is another example function. Any "void" Rust function returning an String is valid.
    fn ls_command() -> String {

        let output = process::Command::new("sh")
            .arg("-c")
            .arg("ls -lh")
            .output()
            .expect("failed to execute process");

        String::from_utf8_lossy(&output.stdout).to_string()
    }
}
