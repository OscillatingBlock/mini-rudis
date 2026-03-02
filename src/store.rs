use crate::connection::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::connection::Frame;

pub struct Store {
    data: Arc<Mutex<HashMap<String, Frame>>>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set(&self, key: String, value: Frame) {
        let mut map = self.data.lock().expect("mutex poisoned");
        map.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Result<Option<Frame>, Error> {
        let map = self.data.lock().expect("mutex poisoned");
        // We clone here because we can't return a reference to something
        // that is protected by a Mutex (the lock drops when the function ends)

        match map.get(key) {
            Some(Frame::Hash(_)) => Err(Error::Protocol(
                "WRONGTYPE Operation against a key holding the wrong kind of value".into(),
            )),
            Some(frame) => Ok(Some(frame.clone())),
            None => Ok(None),
        }
    }

    pub fn hset(&self, key: String, field: String, value: Frame) -> Result<(), Error> {
        let mut map = self.data.lock().expect("mutex poisoned");
        //  Ensure the key exists and is a Hash
        let entry = map
            .entry(key)
            .or_insert_with(|| Frame::Hash(HashMap::new()));

        if let Frame::Hash(inner_map) = entry {
            inner_map.insert(field, value);
            Ok(())
        } else {
            Err(Error::Protocol(
                "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
            ))
        }
    }

    pub fn hget(&self, key: &str, field: &str) -> Result<Option<Frame>, Error> {
        let map = self.data.lock().unwrap();

        match map.get(key) {
            // Case 1 & 2: It's a hash. We return Ok, but the inner value might be None.
            Some(Frame::Hash(inner_map)) => Ok(inner_map.get(field).cloned()),
            // Case 3: Error state.
            Some(_) => Err(Error::Protocol("WRONGTYPE ...".into())),
            // Case 2 (Alternative): Key doesn't exist.
            None => Ok(None),
        }
    }

    pub fn dump(&self, keys: Vec<String>) -> i64 {
        let mut map = self.data.lock().expect("mutex poisoned");
        let mut count = 0;

        for key in keys {
            if map.remove(&key).is_some() {
                count += 1;
            }
        }
        count
    }

    pub fn hdel(&self, key: &str, fields: Vec<String>) -> i64 {
        let mut map = self.data.lock().expect("mutex poisoned");
        let mut count = 0;

        if let Some(Frame::Hash(inner_map)) = map.get_mut(key) {
            for field in fields {
                if inner_map.remove(&field).is_some() {
                    count += 1;
                }
            }
        }
        count
    }
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
