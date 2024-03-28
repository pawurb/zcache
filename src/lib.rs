use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

static mut ZCACHE_STORE: Lazy<HashMap<String, (u128, Box<ZEntry>)>> = Lazy::new(HashMap::new);

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
    pub async fn fetch<F, Fut>(
        key: &str,
        expires_in: Option<Duration>,
        f: F,
    ) -> Result<ZEntry, ZCacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<ZEntry>>,
    {
        match Self::read(key) {
            Some(value) => Ok(value),
            None => match f().await {
                Some(value) => {
                    Self::write(key, value.clone(), expires_in);
                    Ok(value)
                }
                None => Err(ZCacheError::FetchError(key.to_string())),
            },
        }
    }

    pub fn read(key: &str) -> Option<ZEntry> {
        let key = key.to_string();
        let result = unsafe { ZCACHE_STORE.get(&key) };
        match result {
            Some((valid_until, value)) => {
                let valid_until = *valid_until;
                if valid_until == 0 || valid_until > now_in_millis() {
                    Some(*value.clone())
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn write(key: &str, value: ZEntry, expires_in: Option<Duration>) {
        let key = key.to_string();

        let valid_until: u128 = match expires_in {
            Some(duration) => now_in_millis() + duration.as_millis(),
            None => 0,
        };
        unsafe {
            ZCACHE_STORE.insert(key, (valid_until, Box::new(value)));
        }
    }
}

fn now_in_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards!")
        .as_millis()
}

#[cfg(test)]
mod tests {
    use std::ops::Mul;
    use std::thread::sleep;

    use super::*;

    #[tokio::test]
    async fn read_write_works() {
        let cacheable = ZEntry::Int(1);
        let one_second = Duration::from_secs(1);
        ZCache::write("key1", cacheable, Some(one_second));
        let result = ZCache::read("key1");

        match result {
            Some(ZEntry::Int(value)) => assert_eq!(value, 1),
            _ => panic!("Unexpected value"),
        }

        sleep(one_second.mul(2));
        let result = ZCache::read("key1");

        if result.is_some() {
            panic!("Entry should be expired!");
        }

        let cacheable = ZEntry::Text("cached text".to_string());
        ZCache::write("key2", cacheable, None);
        sleep(one_second.mul(2));
        let result = ZCache::read("key2");
        match result {
            Some(ZEntry::Text(value)) => assert_eq!(value, "cached text".to_string()),
            _ => panic!("Unexpected value"),
        }
    }

    #[tokio::test]
    async fn fetch_works() {
        let cacheable = ZEntry::Int(1);
        let result = ZCache::fetch("key1", None, || async { Some(cacheable.clone()) }).await;

        match result {
            Ok(ZEntry::Int(value)) => assert_eq!(value, 1),
            _ => panic!("Unexpected value"),
        }
    }

    #[tokio::test]
    async fn fetch_expiry_works() -> Result<(), ZCacheError> {
        let cacheable = ZEntry::Int(1);
        let one_second = Duration::from_secs(1);
        let result = ZCache::fetch("key1", Some(one_second), || async {
            Some(cacheable.clone())
        })
        .await;
        match result {
            Ok(ZEntry::Int(value)) => assert_eq!(value, 1),
            _ => panic!("Unexpected value"),
        }

        let result = match ZCache::fetch("key1", Some(one_second), || async {
            Some(cacheable.clone())
        })
        .await?
        {
            ZEntry::Int(value) => value,
            _ => panic!("Unexpected type"),
        };

        sleep(one_second.mul(2));
        let result = ZCache::read("key1");

        if result.is_some() {
            panic!("Entry should be expired!");
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_ether_price() -> Result<(), ZCacheError> {
        if let ZEntry::Float(value) =
            ZCache::fetch("ether-price", Some(Duration::from_secs(60)), || async {
                Some(ZEntry::Float(1.1))
            })
            .await?
        {
            println!("Value: {}", value);
            Ok(())
        }
        Ok(())
    }
}
