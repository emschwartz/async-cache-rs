- LRU + explicit expiry (2 apis or one?)
- 2 APIs - get/set + wrapped function
- single vs multi-threaded
- is it a problem that responses may come back significantly after the request
- generic over the request and response types
- don't cache errors

2 sorted lists:
- insertion order
- expiry order


- read/write lock? - might have trouble acquiring write

- wrap reqwest