use std::path::Path;
use std::collections::HashSet;
use rusqlite::{Connection,Result};
use crate::domains::Subdomains;

pub fn initialize_db(db_path: &Path) -> Result<(), rusqlite::Error> {
    let conn = match Connection::open(db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprint!("Could not reach {}: {}",&db_path.to_str().unwrap() , e);
            return Err(e);
        }
    };

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS domain(
          name TEXT PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS subdomain(
          name TEXT PRIMARY KEY,
          active INTEGER,
          parent TEXT,
          FOREIGN KEY(parent) REFERENCES domain(name)
        );")?;
    Ok(())
}

pub fn db_add_domain(data: &Subdomains,connection: &mut Connection) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT OR REPLACE INTO domain(name) VALUES (?)"
        , (&data.domain,))?; 

    let tr = connection.transaction()?;
    {
        let mut sql = tr.prepare(
            "INSERT OR REPLACE INTO subdomain(name,active,parent) VALUES (?1,?2,?3)")?;
        for subdomain in &data.subdomains {
            sql.execute((subdomain, "1", &data.domain))?;
        }
    }

    tr.commit()?;
    Ok(())
}

pub fn get_db_subdomains(domain: &str , connection:&mut Connection) -> rusqlite::Result<HashSet<String>> {
    connection.prepare("
        SELECT name FROM subdomain WHERE parent=?")?
        .query_map([domain],|subdomain| subdomain.get::<_,String>(0))?
        .collect::<Result<HashSet<_>>>()
}

