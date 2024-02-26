#![allow(dead_code)]

pub fn get_opt_dot_env(key: &str, fallback: &str) -> String {
    match dotenv::var(key) {
        Ok(v) => v,
        Err(dotenv::Error::EnvVar(std::env::VarError::NotPresent)) => fallback.to_string(),
        Err(err) => {
            if err.not_found() {
                fallback.to_string()
            } else {
                panic!("get env error {:?}", err)
            }
        }
    }
}

pub fn get_dot_env(key: &str) -> String {
    dotenv::var(key).unwrap()
}
