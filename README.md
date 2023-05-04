# rust-proxybroker
A python proxybroker port written with rust

# Current test
```bash
running 6 tests
  test queue_basic ... ok
  test queue_thread_safety ... ok
  test queue_parallel_async ... ok
  test resolver_resolve_host ... ok
  test resolver_get_real_ext_ip ... ok
  test resolver_get_ip_info ... ok

  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
```

# Todo
- [x] resolver.rs
- [x] judges.rs
- [x] queue
- [ ] providers/
- [ ] negotiator.rs
- [ ] server.rs
- [ ] api.rs

# Note
this project is currently under development
