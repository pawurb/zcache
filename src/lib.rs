use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::future::Future;
use thiserror::Error;

static mut ZCACHE_STORE: Lazy<HashMap<String, Box<ZEntry>>> = Lazy::new(HashMap::new);

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ZCacheError {
    #[error("Failed fetching '{0}' zcache key")]
    FetchError(String),
}

#[derive(Debug, Clone)]
pub enum ZEntry {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}

pub struct ZCache {}

impl ZCache {
    pub async fn fetch<F, Fut>(key: &str, f: F) -> Result<ZEntry, ZCacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<ZEntry>>,
    {
        match Self::read(key) {
            Some(value) => Ok(value),
            None => match f().await {
                Some(value) => {
                    Self::write(key, value.clone());
                    Ok(value)
                }
                None => Err(ZCacheError::FetchError(key.to_string())),
            },
        }
    }

    pub fn read(key: &str) -> Option<ZEntry> {
        let key = key.to_string();
        let result = unsafe { ZCACHE_STORE.get(&key) };
        result.map(|value| *value.clone())
    }

    pub fn write(key: &str, value: ZEntry) {
        let key = key.to_string();
        unsafe {
            ZCACHE_STORE.insert(key, Box::new(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_write_works() {
        let cacheable = ZEntry::Int(1);
        ZCache::write("one", cacheable);
        let result = ZCache::read("one");

        match result {
            Some(ZEntry::Int(value)) => assert_eq!(value, 1),
            _ => panic!("Unexpected value"),
        }

        let cacheable = ZEntry::Text("cached text".to_string());
        ZCache::write("two", cacheable);
        let result = ZCache::read("two");
        match result {
            Some(ZEntry::Text(value)) => assert_eq!(value, "cached text".to_string()),
            _ => panic!("Unexpected value"),
        }
    }

    #[tokio::test]
    async fn fetch_works() {
        let cacheable = ZEntry::Int(1);
        let result = ZCache::fetch("one", || async { Some(cacheable.clone()) }).await;

        match result {
            Ok(ZEntry::Int(value)) => assert_eq!(value, 1),
            _ => panic!("Unexpected value"),
        }
    }
}
