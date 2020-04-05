use std::borrow;
use std::fs;
use std::io::prelude::*;
use std::net;
use std::path;
use std::thread;

type Function = fn() -> String;


#[derive(Clone)]
pub struct Server {
    ip_address: String,
    tcp_port:   u32,
    debug:      bool,
    root_dir:   path::PathBuf,
    std_page:   String,
    dynamic_pages: Vec<DynamicPage>,
}

#[derive(Clone)]
pub struct DynamicPage {
    pub url:     String,
    pub anchors: Vec<Anchor>,
}

#[derive(Clone)]
#[derive(Debug)]
pub struct Anchor {
    pub marker:   String,
    pub function: Function,
}


impl Server {

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

    pub fn debug(&mut self, debug: bool) {
        self.debug = debug;
    }

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

    pub fn std_page(&mut self, std_page: String) {
        self.std_page = std_page;
    }

    pub fn insert_dynamic_page(&mut self, dynamic_page: DynamicPage) {
        self.dynamic_pages.push(dynamic_page);
    }

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
                // if server.debug { eprintln!("{:?}", &dynamic_page); }
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

        let anchor = crate::Anchor {
            marker: "<!-- [[ uptime_content ]] -->".to_string(),
            function: uptime_command,
        };

        let anchor1 = anchor.clone();
        let dynamic_page1 = crate::DynamicPage {
            url: "/".to_string(),
            anchors: vec![anchor1],
        };

        let anchor2 = anchor.clone();
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

    fn uptime_command() -> String {

        let output = process::Command::new("sh")
            .arg("-c")
            .arg("uptime")
            .output()
            .expect("failed to execute process");

        String::from_utf8_lossy(&output.stdout).to_string()
    }
}
