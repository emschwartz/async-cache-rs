# Async Cache

This is an implementation of a simple in-memory cache for use in async applications. It has a utility function to cache the results of a fallible async function.

See the [main function](./src/main.rs) for an example of how to wrap an async function with the cache.

To run the example, run:
- `FLASK_APP=http_server.py flask run`
- `cargo run`

## Design

The [`AsyncCache`](./src/async_cache.rs) wraps a synchronous cache with Tokio's `RwLock` to efficiently support read-heavy workloads. This implementation lazily checks for expired values when a key is fetched.

`AsyncCache.cache_fn` wraps a given function that takes some input and returns a `Future` whose output type is `Result<(Val, Duration), ErrType>`. This provides flexibility for the user to integrate the library into their application in a way that makes sense for their context. The wrapping function will return `Result<Val, ErrType>` and use the `Duration` to determine how long the value should be cached for. Error values are not cached (to cache all or certain types of errors, the `Val` type can be set to a `Result`).

The [`SyncCache`](./src/sync_cache.rs) supports setting items with individual TTLs. It stores values in a `HashMap` and keeps a `PriorityQueue` of key expiry times for efficiently checking for and removing expired items.

## Considerations

### Efficiency for Small Objects

This implementation keeps two copies of the `Key` per object, as well as a UTC timestamp for its expiry. If the use case involves small objects, it may be worth using a system along the lines of Twitter's [SegCache](https://twitter.github.io/pelikan/2021/segcache.html) to minimize the overhead per object.

### Lazy vs Proactive Expiry

The `AsyncCache` lazily removes expired keys when a `get` call is made for a key that is expired. This means that expired objects may stay in memory for significantly longer than necessary. An alternative approach would be to create a background task that removes expired keys as soon as they expire.

### Eviction Strategy

When inserting an item into a cache that is at capacity, this implementation evicts the key expiring soonest. For caches that tend to operate at or near capacity, it may be preferable to evict the least recently used item instead.

### Exposing `chrono` Types in API

The sync and async caches take `chrono::Duration` objects to represent the TTLs. This is preferable to simple integers, because this carries the time unit with it. Although `chrono` is very widely used in the Rust ecosystem, it might be preferable in a production implementation to only use `std::time::Duration` in the public API, or support conditional compilation for `chrono` usage.

## Possible Extensions
- Procedural Macro for caching the results of a function
- Additional helper functions, like an extension to `reqwest` that automatically caches GET requests and respects `Cache-Control` headers
- Make the `AsyncCache` runtime-independent by using an implementation of `RwLock` that supports other runtimes