
# Hike

A bare-bones HTTP server library with dynamic page capabilties.


## Introduction

Hike is designed with only minimum essencial functionality to serve pages via 
HTTP. Hike is based on summit (https://github.com/manoeldesouza/summit) and is 
also conceived with the objective to provide only minimum capabilties.
As summit, it can be better described by what it does not do:

 - Files served are relative to local directory;
 - Either a successful response (200), or Not Found (404) is provided;
 - If a directory is part of the HTTP GET, an index file "index.html" will be 
   assumed instead.


But different to summit, hike provides means to serve dynamic pages with the
usage of Anchors and functions.
 

## Compilation

To play the example static server:

    $ cargo test static_server


Then play the example dynamic server: 

    $ cargo test dynamic_server


## Usage

To start a static server instance:

    let server = hike::Server::new("127.0.0.1".to_string(), 8080);
    server.run();


To start a dynamic server instance:

    let mut server = hike::Server::new("127.0.0.1".to_string(), 8080);
    
    let anchor = crate::Anchor {
        marker: "<!-- [Marker1] -->".to_string(),
        function: <Function1>,
    };
    
    let dynamic_page = crate::DynamicPage {
        url: "/".to_string(),
        anchors: vec![anchor],
    };
    
    server.insert_dynamic_page(dynamic_page);
    
    server.run();


A good reference of how to use library is available in the module tests in 
lib.rs file.


## Author

Manoel de Souza <manoel.desouza@outlook.com.br>

April 2020
