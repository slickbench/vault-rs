use std::collections::HashMap;
use std::io::Read;
// use std::io;

use hyper::Client;
use hyper::client::response::Response;
use hyper::error::Error;
use hyper::header;
// use hyper::header::Connection;
use hyper::status::StatusCode;

use rustc_serialize::json;
use rustc_serialize::json::DecoderError;

pub struct VaultClient<'a> {
    pub hosts: Vec<&'a str>,
    pub token: &'a str,
    client: Client,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct SecretData {
    value: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct SecretAuth {
    client_token: String,
    policies: Vec<String>,
    metadata: HashMap<String, String>,
    lease_duration: i64,
    renewable: bool,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct VaultSecret {
    lease_id: Option<String>,
    renewable: Option<bool>,
    lease_duration: i64,
    data: SecretData,
    warnings: Option<Vec<String>>,
    auth: Option<SecretAuth>,
}

header! { (XVaultToken, "X-Vault-Token") => [String] }

impl<'a> VaultClient<'a> {
    pub fn new(hosts: Vec<&'a str>, token: &'a str) -> Result<VaultClient<'a>, String> {

        let client = Client::new();
        for host in &hosts {
            match client.get(&format!("{}/v1/auth/token/lookup-self", host)[..])
                .header(XVaultToken(token.to_string()))
                .send() {
                    Ok(s) => {
                        match s.status {
                            StatusCode::Forbidden => return Err("Forbidden".to_string()),
                            _ => { break }
                        }

                    },
                    // Err(Error { kind: ConnectionRefused }) => continue,
                    Err(e) => {
                        match e {
                            Error::Io(_) => continue,
                            _ => return Err(format!("{:?}", e)),
                        }
                    }

                }
            }
        Ok(VaultClient {
            hosts: hosts,
            token: token,
            client: client,
        })
    }

    ///
    /// Saves a secret
    ///
    /// ```
    /// # extern crate hashicorp_vault as vault;
    /// # use vault::Client;
    /// # fn main() {
    /// let hosts = vec!["http://127.0.0.1:8200"];
    /// let token = "test12345";
    /// let client = Client::new(hosts, token).unwrap();
    /// let res = client.set_secret("hello", "world");
    /// assert!(res.is_ok());
    /// # }
    /// ```

    pub fn set_secret(&self, key: &str, value: &str) -> Result<&str, &str> {
        match self.post(&format!("/v1/secret/{}", key)[..], &format!("{{\"value\": \"{}\"}}", value)[..]) {
            Ok(s) => {
                match s.status {
                    StatusCode::NoContent => Ok(""),
                    _ => { Err("Error setting secret")}
                }
            },
            Err(e) => {
                println!("{:?}", e);
                Err("err")
            }
        }
    }

    ///
    /// Fetches a saved secret
    ///
    /// ```
    /// # extern crate hashicorp_vault as vault;
    /// # use vault::Client;
    /// # fn main() {
    /// let hosts = vec!["http://127.0.0.1:8200"];
    /// let token = "test12345";
    /// let client = Client::new(hosts, token).unwrap();
    /// let res = client.set_secret("hello", "world");
    /// assert!(res.is_ok());
    /// let res = client.get_secret("hello");
    /// assert!(res.is_ok());
    /// assert_eq!(res.unwrap(), "world");
    /// # }
    /// ```

    pub fn get_secret(&self, key: &str) -> Result<String, &str> {
        match self.get(&format!("/v1/secret/{}", key)[..]) {
            Ok(mut s) => {
                let mut body = String::new();
                s.read_to_string(&mut body).unwrap();
                let decoded: Result<VaultSecret, DecoderError> = json::decode(&body);
                match decoded {
                    Ok(decoded) => {
                        let d: SecretData = decoded.data;
                        Ok(d.value)
                    },
                    Err(e) => {
                        println!("Error: {:?}", e);
                        Err("Got a bad secret back")
                    }
                }
            },
            Err(e) => {
                println!("Error: {:?}", e);
                Err("err")
            }
        }
    }

    ///
    /// Deletes a saved secret
    ///
    /// ```
    /// # extern crate hashicorp_vault as vault;
    /// # use vault::Client;
    /// # fn main() {
    /// let hosts = vec!["http://127.0.0.1:8200"];
    /// let token = "test12345";
    /// let client = Client::new(hosts, token).unwrap();
    /// let res = client.set_secret("hello", "world");
    /// assert!(res.is_ok());
    /// let res = client.delete_secret("hello");
    /// assert!(res.is_ok());
    /// # }
    /// ```
    pub fn delete_secret(&self, key: &str) -> Result<&str, &str> {
        match self.delete(&format!("/v1/secret/{}", key)[..]) {
            Ok(s) => {
                match s.status {
                    StatusCode::NoContent => Ok(""),
                    _ => { Err("Error setting secret")}
                }
            },
            Err(e) => {
                println!("{:?}", e);
                Err("err")
            }
        }
    }

    fn get(&self, endpoint: &str) -> Result<Response, String> {
        for host in &self.hosts {
            match self.client.get(&format!("{}{}", host, endpoint)[..])
                .header(XVaultToken(self.token.to_string()))
                .header(header::ContentType::json())
                .send() {
                    Ok(s) => return Ok(s),
                    // Err(Error { kind: ConnectionRefused }) => continue,
                    Err(e) => {
                        match e {
                            Error::Io(_) => continue,
                            _ => return Err(format!("{:?}", e)),
                        }
                    }
                }
        }
        Err("No working host".to_string())
    }

    fn delete(&self, endpoint: &str) -> Result<Response, String> {
        for host in &self.hosts {
            match self.client.delete(&format!("{}{}", host, endpoint)[..])
                .header(XVaultToken(self.token.to_string()))
                .header(header::ContentType::json())
                .send() {
                    Ok(s) => return Ok(s),
                    // Err(Error { kind: ConnectionRefused }) => continue,
                    Err(e) => {
                        match e {
                            Error::Io(_) => continue,
                            _ => return Err(format!("{:?}", e)),
                        }
                    }
                }
        }
        Err("No working host".to_string())
    }

    fn post(&self, endpoint: &str, body: &str) -> Result<Response, String> {
        for host in &self.hosts {
            match self.client.post(&format!("{}{}", host, endpoint)[..])
                .header(XVaultToken(self.token.to_string()))
                .header(header::ContentType::json())
                .body(body)
                .send() {
                    Ok(s) => return Ok(s),
                    // Err(Error { kind: ConnectionRefused }) => continue,
                    Err(e) => {
                        match e {
                            Error::Io(_) => continue,
                            _ => return Err(format!("{:?}", e)),
                        }
                    }
                }
        }
        Err("No working host".to_string())
    }
    // fn get_new_host(&self) -> usize {
    //     let mut rng = thread_rng();
    //     rng.gen_range(0, hosts.len() as u32 - 1)
    // }
}

