# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

<!-- insertion marker -->
## [0.1.0](https://github.com/zevtyardt/proxy.rs/releases/tag/0.1.0) - 2023-05-11

<small>[Compare with first commit](https://github.com/zevtyardt/proxy.rs/compare/ef19c602773cd7f0494ab9bfc2a832111202c8ff...0.1.0)</small>

### Tests

- added test function for resolver and queue ([10d2778](https://github.com/zevtyardt/proxy.rs/commit/10d2778bca57a6fef7fe693738ae67c4c3199af6) by zevtyardt).
- test resolves 3 times ([f591115](https://github.com/zevtyardt/proxy.rs/commit/f591115c3aa2e2b1b56c6428549fd65f624db582) by zevtyardt).
- added geo test and host resolver ([1260fa1](https://github.com/zevtyardt/proxy.rs/commit/1260fa1fca71e16335a82fc313fb81a278902bc7) by zevtyardt).

### Chore

- update log statements in geolite_database.rs ([40622a4](https://github.com/zevtyardt/proxy.rs/commit/40622a4e3c89aa8dee55a9d8b1f3ce5bfc7386f2) by zevtyardt).
- rename pkg to proxy-rs ([161ad6b](https://github.com/zevtyardt/proxy.rs/commit/161ad6bf18eb380ac18299a1143b62e54b31af88) by zevtyardt).
- update main.rs ([8953fb5](https://github.com/zevtyardt/proxy.rs/commit/8953fb5dea5e5662b73ec8ad707613a88cf1cc33) by zevtyardt).
- make  resolve function runs within the tokio runtime independently ([bf843ab](https://github.com/zevtyardt/proxy.rs/commit/bf843ab190c74a02507db3807c48e2152ee1fd5d) by zevtyardt).
- update geolite database url location ([f00ede3](https://github.com/zevtyardt/proxy.rs/commit/f00ede39f43b198a7aaa4172de4e683af93b269e) by zevtyardt).
- move the test function to its own file ([c214be5](https://github.com/zevtyardt/proxy.rs/commit/c214be5601e1fe79279377201e339097322e5061) by zevtyardt).
- delete changelog.md ([a6bb110](https://github.com/zevtyardt/proxy.rs/commit/a6bb1100079d8b818b65334d49777a27dd0fea76) by zevtyardt).
- delete workflows ([25a6161](https://github.com/zevtyardt/proxy.rs/commit/25a616103e250d9d261cc8a44e7a889a27d3fe38) by zevtyardt).
- update changelog.yml ([e09aa76](https://github.com/zevtyardt/proxy.rs/commit/e09aa7671bc759ac78631b5768fdcc9556fdabc6) by zevtyardt).

### Bug Fixes

- Merge branch 'main' of https://github.com/zevtyardt/rust-proxybroker ([6a75756](https://github.com/zevtyardt/proxy.rs/commit/6a75756fb3a6adbed5b9922065f0fc7f248d2fe4) by zevtyardt).

### Features

- add Proxy module and update logging configuration ([034fbf0](https://github.com/zevtyardt/proxy.rs/commit/034fbf0aa1167fb61a7311da7d19dbdea35c4a56) by zevtyardt).
- Add `Default` trait implementation to `GeoData` ([e8e1f84](https://github.com/zevtyardt/proxy.rs/commit/e8e1f84de97a9bbaf1da9344c4508f6763ad3213) by zevtyardt).
- Add geo data to Proxy struct ([b80eea9](https://github.com/zevtyardt/proxy.rs/commit/b80eea9f5762974c3da6a6366ca03f88bc05f67b) by zevtyardt).
- added Proxy struct ([79d8069](https://github.com/zevtyardt/proxy.rs/commit/79d8069e92ecf7f3bacfa4574c728b852740c932) by zevtyardt).
- added basic API core functionality ([366a15f](https://github.com/zevtyardt/proxy.rs/commit/366a15f6638793ff7e2a723988e813ed82cd4885) by zevtyardt).
- added 2 new dependencies: rand & regex ([956345a](https://github.com/zevtyardt/proxy.rs/commit/956345aacae970cf182df58e3d38e3c153253241) by zevtyardt).
- added the random_useragent function to the utils module ([7e4f277](https://github.com/zevtyardt/proxy.rs/commit/7e4f277fc36c813811cfef8e1406e197bbb41b66) by zevtyardt).
- added free-proxy-list.net proxy scraper ([69c576e](https://github.com/zevtyardt/proxy.rs/commit/69c576e7d52b51480206ee79c703377d0df699fe) by zevtyardt).
- added base provider basic functionality ([ef500d1](https://github.com/zevtyardt/proxy.rs/commit/ef500d19480e35412020e604ee146b0fafc2235c) by zevtyardt).
- added timeout and verify_ssl for the Judge struct ([d75bfdf](https://github.com/zevtyardt/proxy.rs/commit/d75bfdf139b7f5837af7f332f354b20fe1d944fd) by zevtyardt).
- implements Display for the FifoQueue struct ([8a179b6](https://github.com/zevtyardt/proxy.rs/commit/8a179b64d7acb6135191160481ca851d71d1ad93) by zevtyardt).
- added a custom type for async futures ([d0e7cd8](https://github.com/zevtyardt/proxy.rs/commit/d0e7cd84aaefd5961bc0256f590d7c8e9c8a93f2) by zevtyardt).
- added logging for better output ([721b4e8](https://github.com/zevtyardt/proxy.rs/commit/721b4e8e283ab95bc9118d9ad779f48b5154d4e3) by zevtyardt).
- added some dependencies for logging purposes ([a582e0c](https://github.com/zevtyardt/proxy.rs/commit/a582e0c3004eb55b3fc3f8a0dfed53980ed92627) by zevtyardt).
- added a simple queue implementation ([df390a3](https://github.com/zevtyardt/proxy.rs/commit/df390a333894eb3e0f5b0282e5deb1d7db19e3ae) by zevtyardt).
- add changelog ([8da12b5](https://github.com/zevtyardt/proxy.rs/commit/8da12b5086914e05c724ced50cc11c86184c4181) by zevtyardt).
- add github actions to auto generate changelog ([8d1c6db](https://github.com/zevtyardt/proxy.rs/commit/8d1c6dbb901bc8a0fe80edc47fae6ad9eb36af9d) by zevtyardt).

### Docs

- update changelog.md ([902c2fc](https://github.com/zevtyardt/proxy.rs/commit/902c2fc7c16bfe3c8f2628183b4a5fd705ff51e6) by zevtyardt).
- added 2 new items to todo section ([3a7ab99](https://github.com/zevtyardt/proxy.rs/commit/3a7ab9911a5ecdd95074f100ad1d60cd5938f0b4) by zevtyardt).
- added current test result and queue ([4d8e094](https://github.com/zevtyardt/proxy.rs/commit/4d8e094d07364ce79195372dc76b5c1649e86817) by zevtyardt).
- update release notes ([076c004](https://github.com/zevtyardt/proxy.rs/commit/076c004ec4a2464ba6f9499e625680ff301f1965) by zevtyardt).

