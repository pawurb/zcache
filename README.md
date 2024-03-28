# zcache [![Latest Version](https://img.shields.io/crates/v/zcache.svg)](https://crates.io/crates/zcache) [![GH Actions](https://github.com/pawurb/zcache/actions/workflows/ci.yml/badge.svg)](https://github.com/pawurb/zcache/actions)

Zcache is an in-memory cache store with time-based expiration. This project aims to provide a straightforward API to cache any part of a Rust application without a need to modify other parts of the callstack.

## Usage

You can cache `ZEntry` enum variants that encapsulate primitive types:

```rust
enum ZEntry {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}
```

`ZCache` module exposes `fetch`, `read`, `write` and `clear` methods:

### `fetch`

`fetch` accepts the name of the cache key, optional expiry time, and async callback, used to populate the cache if it is missing or expired:

```rust

async fn get_ether_price() -> Result<f64> {
  match ZCache::fetch("ether-price", Some(Duration::from_secs(60)), || async {
      let price: f64 = json_client.get().await...
      // logic to extract price from URL ...

      Some(ZEntry::Float(price))
  })
  .await? {
      ZEntry::Float(price) => Ok(price),
      _ => panic!("Unexpected type!"),
  }
}

```

In the above implementation, `get_ether_price` returns the price fetched from a URL. It triggers the HTTP request only once every 60 seconds.

One limitation is that async callback cannot return an `Err` so you must communicate failures in cache refresh by returning `None`. 

### `read` and `write` 

```rust
async fn refresh_ether_price() -> Result<()> {
  let price: f64 = json_client.get().await...
  let price = ZEntry::Float(price);

  ZCache::write("ether-price", price, Some(Duration::from_secs(60)))
  Ok(())
}

fn get_ether_price() -> Some(f64) {
    ZCache::read("ether-price", price)
}
```

In the above example, the async function `write` can periodically refresh price fetched from an URL. The advantage of `read` over `fetch` is that it's not `async`, so it's possible to use it in non-async parts of your application.

### `clear` 

```rust
  ZCache::clear();
```

Use it to remove all the cache entires.

## Status

All these methods are just fancy wrappers over `unsafe` mutable static variable, so proceed with caution. Data races in multithreaded environments are expected. But, since I'm using it only for caching, I assumed it's acceptable. 

I'm using `zcache` in a production app, but please treat it as proof of concept. I have limited Rust experience, so feedback is appreciated.

**[Update]** it seems to be randomly segfaulting, so better don't use it.
