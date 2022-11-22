use std::{
    io::prelude::*,
    collections::HashMap,
    str::from_utf8,
    string::String,
    net::{SocketAddr, UdpSocket, TcpStream},
    sync::{Arc,Mutex},
    thread,
};

use crate::{
    dns_parse::{server_config_parse, domain_database_parse},
    dns_structs::{
        dns_message::DNSMessage, domain_database_struct::DomainDatabase,
        server_config::ServerConfig,
    },
};

pub fn start_ss(config_path: String, port: u16) {
    let config: ServerConfig = match server_config_parse::get(config_path) {
        Ok(config) => config,
        Err(_err) => panic!("Server config path not found!"),
    };
    let db : HashMap<String,DomainDatabase> = HashMap::new();
    
    let mut mutable_db : Arc<Mutex<HashMap<String,DomainDatabase>>>  = Arc::new(Mutex::new(db));
    // ver o ip do sp -> criar thread para fazer o pedido  
    for (domain_name, domain_config) in config.get_domain_configs().iter() {
        let sp_addr = match domain_config.get_domain_sp() {
            Some(addr) => addr,
            None => {
                println!("SP not found for {} skipping domain", domain_name);
                continue
            }
        };
        let handler = thread::spawn(move || db_sync(domain_name.to_string(), sp_addr, Arc::clone(&mutable_db)));
    }
}

fn db_sync(domain_name: String, sp_addr: SocketAddr, db: Arc<Mutex<HashMap<String,DomainDatabase>>>) {
    
    let mut tcp_stream = match TcpStream::connect(sp_addr) {
        Ok(stream) => stream,
        Err(err) => {panic!("Could't connect to addr {}", sp_addr);}
    };

    tcp_stream.write(domain_name.as_bytes());

    let mut buf = &mut [0u8;1000];

    tcp_stream.read(buf);
    
    let entries: u16 = (buf[0].to_owned() as u16 * 256) + buf[1].to_owned() as u16 ;
    
    // confirmacao resolver isto ... 
    tcp_stream.write(buf);
    
    let mut unparsed_db: Vec<&str> = Vec::with_capacity(entries.try_into().unwrap()); 
    // codificao primeiros 2 bytes sao o numero de ordem da entry o resto e do tipo Entry
    for i in 0..entries { 
        let mut seq_number_bin = &mut [0u8,2];
        tcp_stream.take(2).read(seq_number_bin);
        let seq_number: u16 = (seq_number_bin[0] as u16 * 256) + seq_number_bin[1] as u16;

        tcp_stream.read(buf);
        let line = from_utf8(buf).unwrap();
        unparsed_db.insert(i.try_into().unwrap(), line);
    };
    let db_txt: String = String::new(); 

    for line in unparsed_db {
        db_txt.push_str(line);
    }
    let domain_db: DomainDatabase = match domain_database_parse::parse_from_str(db_txt) {
        Ok(db) => db,
        Err(err) => panic!("Coudn't parse database")
    };
    
    let mut locked_db = db.lock().unwrap();
    locked_db.insert(domain_name,domain_db);
}
