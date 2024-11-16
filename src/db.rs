use std::path::Path;

use rusqlite::Connection;

pub fn initialize_db(db_path: &Path) -> Result<(), rusqlite::Error> {
    let conn = match Connection::open(db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprint!("Could not reach {}: {}",&db_path.to_str().unwrap() , e);
            return Err(e);
        }
    };

    conn.execute_batch("
        CREATE TABLE domain(
          name TEXT PRIMARY KEY
        );

        CREATE TABLE subdomain(
          name TEXT PRIMARY KEY,
          active INTEGER,
          parent TEXT,
          FOREIGN KEY(parent) REFERENCES domain(name)
        );")?;
    Ok(())
}

pub fn db_add_domain(connection: Connection) {
    
}
