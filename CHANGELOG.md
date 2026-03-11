# Changelog

All notable changes to noq will be documented in this file.

## [0.9.0](https://github.com/n0-computer/noq/compare/iroh-quinn-v0.16.1..v0.9.0) - 2026-03-09

### ⛰️  Features

- *(proto)* [**breaking**] Don't require a HKDF construction in `HandshakeTokenKey` ([#480](https://github.com/n0-computer/noq/issues/480)) - ([bb46490](https://github.com/n0-computer/noq/commit/bb46490ea71004e777bc1e3e22af6cf8dcff811c))
- Retain final path stats if a `Path` is alive, add `WeakPathHandle` ([#386](https://github.com/n0-computer/noq/issues/386)) - ([b068cda](https://github.com/n0-computer/noq/commit/b068cda8edd18e2bf412b0ef3b66b0093093d2e9))
- Switch from rand data to path challenges for nat traversal ([#373](https://github.com/n0-computer/noq/issues/373)) - ([c369b52](https://github.com/n0-computer/noq/commit/c369b525bebe84b7fdaddbd6b63cb190d1146c30))
- [**breaking**] Allow compiling with `rustls`, but without any crypto providers compiled into rustls ([#462](https://github.com/n0-computer/noq/issues/462)) - ([13a1c45](https://github.com/n0-computer/noq/commit/13a1c456f543cd9d8bfa58ca3b0ea890a678efb4))
- Add minimal socket2 based impl (previously fallback.rs) ([#478](https://github.com/n0-computer/noq/issues/478)) - ([0185176](https://github.com/n0-computer/noq/commit/018517663dd974a48d0a9eca0b3ff355b980a4ae))

### 🐛 Bug Fixes

- *(ci)* Daily jobs deps ([#413](https://github.com/n0-computer/noq/issues/413)) - ([093cf62](https://github.com/n0-computer/noq/commit/093cf62908c6d23c199192c03dfaa781a6aafa57))
- *(ci)* Address dns issues in daily job ([#416](https://github.com/n0-computer/noq/issues/416)) - ([325db0f](https://github.com/n0-computer/noq/commit/325db0f070536b6fe18e6a76edc43c84d1cc4e9d))
- *(proto)* Don't allow closing paths without multipath ([#387](https://github.com/n0-computer/noq/issues/387)) - ([e7c23e9](https://github.com/n0-computer/noq/commit/e7c23e94e6edf835cb0f40c3ddbccb7937849328))
- *(proto)* Some harmless bugs regarding confidentiality limits ([#423](https://github.com/n0-computer/noq/issues/423)) - ([ebbd765](https://github.com/n0-computer/noq/commit/ebbd7654658415a3c5e627f0c0113e6abc2ac020))
- *(proto)* Handle duplicated reach out frames ([#430](https://github.com/n0-computer/noq/issues/430)) - ([220dacb](https://github.com/n0-computer/noq/commit/220dacbeed5e16247eda5858f05bf6f35628b99c))
- *(proto)* Avoid generating protocol violation errors in bad network conditions ([#436](https://github.com/n0-computer/noq/issues/436)) - ([2903b55](https://github.com/n0-computer/noq/commit/2903b55dded0e688c31a975f4809380a4185a559))
- *(proto)* Fix checks to understand if a path response is valid ([#443](https://github.com/n0-computer/noq/issues/443)) - ([8fc9cdd](https://github.com/n0-computer/noq/commit/8fc9cdd77a3e1ada61388f859d3c1322b1a46294))
- *(proto)* Properly separate on-path and off-path challenge logic ([#449](https://github.com/n0-computer/noq/issues/449)) - ([b966872](https://github.com/n0-computer/noq/commit/b966872d35a9ad099a7d953a8e44583ebbcfb574))
- *(proto)* Remove race condition between `take_error` & overwriting in `move_to_draining` ([#452](https://github.com/n0-computer/noq/issues/452)) - ([6ec9ffe](https://github.com/n0-computer/noq/commit/6ec9ffec6a76e20f54018a498c376a265008985a))
- *(proto)* Set open path timer when first packet is sent ([#458](https://github.com/n0-computer/noq/issues/458)) - ([8a9a702](https://github.com/n0-computer/noq/commit/8a9a7021fdcfde53248ebecd35386922df4de08a))
- *(proto)* Don't generate endpoint events in drained connection state ([#470](https://github.com/n0-computer/noq/issues/470)) - ([ee63d4b](https://github.com/n0-computer/noq/commit/ee63d4bc48c3f96bd02c6bf4ff33ce2efe4ed8c9))
- *(proto)* Avoid unwrapping `VarInt` decoding during `TransportParameter` parsing ([#485](https://github.com/n0-computer/noq/issues/485)) - ([752588b](https://github.com/n0-computer/noq/commit/752588b9801cc4c5fd3064b845a3e71d8240036e))
- *(quinn-proto)* Path abandon does not clear all timers, in particular, not loss detection ([#438](https://github.com/n0-computer/noq/issues/438)) - ([c69a939](https://github.com/n0-computer/noq/commit/c69a939edb0a37119dce98ae1789eed0e36e4717))
- *(quinn-udp)* More wine fixes ([#414](https://github.com/n0-computer/noq/issues/414)) - ([708a6a0](https://github.com/n0-computer/noq/commit/708a6a0119f88039cc24a734f579d8a1c78263a4))
- *(udp)* Windows: make potentially non available socket options optional ([#392](https://github.com/n0-computer/noq/issues/392)) - ([eda1e01](https://github.com/n0-computer/noq/commit/eda1e0164402aede0cd8bee9aa1a6e1e1a61da06))
- Allow the remote to abandon paths even if no validated paths remain ([#401](https://github.com/n0-computer/noq/issues/401)) - ([8289585](https://github.com/n0-computer/noq/commit/8289585713239d87aa30d9fa5375af907d00cac9))
- Update time dep to address RUSTSEC-2026-0009 ([#412](https://github.com/n0-computer/noq/issues/412)) - ([1a91e0b](https://github.com/n0-computer/noq/commit/1a91e0b538fa54bb8aee29e9d6fc7208808fa61d))
- Handle network changes in multipath ([#383](https://github.com/n0-computer/noq/issues/383)) - ([d5580a5](https://github.com/n0-computer/noq/commit/d5580a52928327a16ca9c0901d88cc9396f1b284))
- Open_path_ensure deadlock ([#424](https://github.com/n0-computer/noq/issues/424)) - ([315f491](https://github.com/n0-computer/noq/commit/315f491bfbdd0bb04fb8cdb43967b40bfef83d22))
- CidQueue out of bounds panic ([#431](https://github.com/n0-computer/noq/issues/431)) - ([653d6ee](https://github.com/n0-computer/noq/commit/653d6ee268dd9edf5577140292b3bd5503b8681a))
- Avoid lock re-entry in open_path_ensure and add regression test ([#464](https://github.com/n0-computer/noq/issues/464)) - ([6c4de85](https://github.com/n0-computer/noq/commit/6c4de8557ef48ec1eb24dcdbb0a720d7da43fb7e))

### 🚜 Refactor

- *(proto)* Introduce `CryptoState` ([#420](https://github.com/n0-computer/noq/issues/420)) - ([4f8afee](https://github.com/n0-computer/noq/commit/4f8afee29ff86cd5f6c4fe5567ea182d5af6eef7))
- *(proto)* Expand use of `SpaceKind` where `SpaceId::Data(PathId)` is not suitable ([#432](https://github.com/n0-computer/noq/issues/432)) - ([20e1fcc](https://github.com/n0-computer/noq/commit/20e1fcc073924f7daab68490d5f89806607fd6ff))
- Use named future for SendStream::stopped ([#409](https://github.com/n0-computer/noq/issues/409)) - ([f89efda](https://github.com/n0-computer/noq/commit/f89efda9d66d58021c38279a98e9f5a43b501eae))
- Stop EndpointDriver once endpoint is closed and all connections are drained ([#426](https://github.com/n0-computer/noq/issues/426)) - ([c51afae](https://github.com/n0-computer/noq/commit/c51afaeb6bb10d0a37fc7b36846fe61bc953d556))
- [**breaking**] Improve path events around path closing ([#427](https://github.com/n0-computer/noq/issues/427)) - ([88cf95f](https://github.com/n0-computer/noq/commit/88cf95fcd038da9acc5fc5435d5841837b5327e2))
- Defensive styel & move code around ([#450](https://github.com/n0-computer/noq/issues/450)) - ([0696c83](https://github.com/n0-computer/noq/commit/0696c830d8d751c875c01c74daa53368e890cbd3))
- Remove needless variable ([#453](https://github.com/n0-computer/noq/issues/453)) - ([588efa5](https://github.com/n0-computer/noq/commit/588efa507c824623d5736b9bca9cf2e410facf55))
- Switch back to pending name for this ([#460](https://github.com/n0-computer/noq/issues/460)) - ([6f06f1f](https://github.com/n0-computer/noq/commit/6f06f1f3bdbf7250b35c76d78466e8bf93521327))
- [**breaking**] Rename to noq ([#461](https://github.com/n0-computer/noq/issues/461)) - ([294e3ea](https://github.com/n0-computer/noq/commit/294e3ea603a2c8f86d05c486082b96f1175ad5e5))

### 🧪 Testing

- Document proptest interactions ([#406](https://github.com/n0-computer/noq/issues/406)) - ([8c146d2](https://github.com/n0-computer/noq/commit/8c146d2da4c0d22a5c31ac2c31301f5194b9bdc5))
- Introduce `ConnPair` and `testresult` to simplify tests ([#408](https://github.com/n0-computer/noq/issues/408)) - ([7cabfe8](https://github.com/n0-computer/noq/commit/7cabfe8392060481c81bbbe4ee837fbcb1d84c99))

### ⚙️ Miscellaneous Tasks

- *(ci)* Adjust runner usage and storage policies ([#471](https://github.com/n0-computer/noq/issues/471)) - ([caf81fd](https://github.com/n0-computer/noq/commit/caf81fd2e212bc8baa55a11128147d0feeb4f4b4))
- *(proto)* Fix unused import warning with only `rustls` + `platform-verifier` ([#473](https://github.com/n0-computer/noq/issues/473)) - ([6f48f3a](https://github.com/n0-computer/noq/commit/6f48f3ab4a6fef9b313fda00f8bfbb07efccb5ca))
- Add testing on wine ([#393](https://github.com/n0-computer/noq/issues/393)) - ([c332d42](https://github.com/n0-computer/noq/commit/c332d420ce0ddf5cd8bb781e81c4cb0ea6a7f7bc))
- Add expanded proptest runs to ci ([#395](https://github.com/n0-computer/noq/issues/395)) - ([f41f278](https://github.com/n0-computer/noq/commit/f41f278609c1b0de040d1ab2ae73dc826ec50daf))
- Fix daily proptest runs ([#441](https://github.com/n0-computer/noq/issues/441)) - ([041777b](https://github.com/n0-computer/noq/commit/041777b9d2a940da525845f1deed230d65ca17e6))
- Unify nat traversal naming ([#445](https://github.com/n0-computer/noq/issues/445)) - ([565ebec](https://github.com/n0-computer/noq/commit/565ebec2c0f777e602e692cf07a74b036d62068b))
- Attempt at drafting a readme ([#467](https://github.com/n0-computer/noq/issues/467)) - ([e71b78b](https://github.com/n0-computer/noq/commit/e71b78b88a9d44eee6d7aaa61f1ad2b603fbf6d4))
- Release prep - ([13695a4](https://github.com/n0-computer/noq/commit/13695a47ab1d0c151c536e0f3e5c07b80b315c44))
- Release - ([faeddf5](https://github.com/n0-computer/noq/commit/faeddf58eed8b9b30a153aed5d9acee570934837))


