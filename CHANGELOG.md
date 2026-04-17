# Changelog

All notable changes to noq will be documented in this file.

## [0.18.0](https://github.com/n0-computer/noq/compare/v0.17.0..0.18.0) - 2026-04-15

### ⛰️  Features

- *(noq)* Unify waking the state ([#541](https://github.com/n0-computer/noq/issues/541)) - ([5fd0ad1](https://github.com/n0-computer/noq/commit/5fd0ad1276b6023f01945be752901f9407b6a8d5))
- *(proto)* Add `QuicServerConfig::set_alpn_protocols` in rustls feature ([#496](https://github.com/n0-computer/noq/issues/496)) - ([c21decd](https://github.com/n0-computer/noq/commit/c21decd159f3605e91b796591f7c360ec62e54e4))
- *(proto)* Issue CIDs in order of ascending path ID ([#504](https://github.com/n0-computer/noq/issues/504)) - ([9e78aad](https://github.com/n0-computer/noq/commit/9e78aad8c8bd92211d1e51b4949a7ffa685295ef))
- *(proto)* Send PATH_ABANDON on the abandoned path itself ([#509](https://github.com/n0-computer/noq/issues/509)) - ([fc6f4d9](https://github.com/n0-computer/noq/commit/fc6f4d9ed366ca3254bd591738f0d1c92bd49fdd))
- *(proto)* Add lost_packets and lost_bytes to the ConnectionStats ([#560](https://github.com/n0-computer/noq/issues/560)) - ([ab042ea](https://github.com/n0-computer/noq/commit/ab042ea7e3f7bae1e43eaba2ac9dba47ddb52e69))
- *(proto)* Server sends NAT traversal probes with active CID ([#575](https://github.com/n0-computer/noq/issues/575)) - ([47f61ed](https://github.com/n0-computer/noq/commit/47f61edcc804e642c7b40ed90f6d911848922b5e))

### 🐛 Bug Fixes

- *(ci)* Switch to sccache-action entirely ([#501](https://github.com/n0-computer/noq/issues/501)) - ([89a9e7f](https://github.com/n0-computer/noq/commit/89a9e7f927d7f3b29f28d65d4d59cd0da56bc03f))
- *(proto)* Correctly ignore packets of previous paths ([#520](https://github.com/n0-computer/noq/issues/520)) - ([1833118](https://github.com/n0-computer/noq/commit/1833118875e7ed2006430420bca821ee8fccb3e8))
- *(proto)* [**breaking**] Ensure network paths have cleaned socket addresses ([#513](https://github.com/n0-computer/noq/issues/513)) - ([b3b50c0](https://github.com/n0-computer/noq/commit/b3b50c0b492734e903e3f2fab761d908dab6d1c4))
- *(proto)* Rename saved proptests seeds ([#529](https://github.com/n0-computer/noq/issues/529)) - ([e1c12a6](https://github.com/n0-computer/noq/commit/e1c12a6f4837fcc312223af4368067f50b4926e7))
- *(proto)* Do not ignore ACKs for abandoned paths ([#519](https://github.com/n0-computer/noq/issues/519)) - ([e4649f8](https://github.com/n0-computer/noq/commit/e4649f8e2b729917d29efdfa9c549fc456708ceb))
- *(proto)* Accept remote PATH_ABANDON for last path ([#522](https://github.com/n0-computer/noq/issues/522)) - ([64bad63](https://github.com/n0-computer/noq/commit/64bad63f2febb6af4a5b58c705a7b44f17764390))
- *(proto)* Read network path before clearing local_ip in handle_network_change ([#546](https://github.com/n0-computer/noq/issues/546)) - ([2622b67](https://github.com/n0-computer/noq/commit/2622b677484b4196fc3d20426f911551ea27d315))
- *(proto)* Re-arm PathIdle timer when idle timeout is changed ([#544](https://github.com/n0-computer/noq/issues/544)) - ([cdde4e9](https://github.com/n0-computer/noq/commit/cdde4e92f34b73f2ac63c249a29510844c13616f))
- *(proto)* Reset PTO backoff for recoverable paths on network change ([#545](https://github.com/n0-computer/noq/issues/545)) - ([b41b57a](https://github.com/n0-computer/noq/commit/b41b57a11ef10300764d3862b8aa4f5a049ebc9a))
- *(proto)* Prioritise ADD_ADDRESS and REMOVE_ADDRESS before STREAM ([#550](https://github.com/n0-computer/noq/issues/550)) - ([a75c541](https://github.com/n0-computer/noq/commit/a75c541fd4d7d4f8619b3d2817854192fadd7cd2))
- *(proto)* Cap PTO backoff at 2 seconds post-handshake ([#523](https://github.com/n0-computer/noq/issues/523)) - ([50faca4](https://github.com/n0-computer/noq/commit/50faca414604af2c6f3b880c7cf168e9fffd91bb))
- *(proto)* Display for CONNECTION_CLOSE frame ([#554](https://github.com/n0-computer/noq/issues/554)) - ([c2103a1](https://github.com/n0-computer/noq/commit/c2103a144cdc61f35bc724f353b3316b91e4be5d))
- *(proto)* Retry off-path NAT traversal probes and retire stale CIDs ([#524](https://github.com/n0-computer/noq/issues/524)) - ([7d60937](https://github.com/n0-computer/noq/commit/7d6093715845f53c26c3a185ac4b4948704d2663))
- *(proto)* Tail-loss probes should always be ack-eliciting ([#561](https://github.com/n0-computer/noq/issues/561)) - ([1948efc](https://github.com/n0-computer/noq/commit/1948efc817887037593f5dcc75557f4b0415d0cc))
- *(proto)* Track permit_idle_reset per-path ([#576](https://github.com/n0-computer/noq/issues/576)) - ([96ecc99](https://github.com/n0-computer/noq/commit/96ecc99d3eeb9705e2dbdf39eecc2cc96deda7d1))
- *(proto)* Avoid scheduling tail-loss probes on abandoned paths ([#562](https://github.com/n0-computer/noq/issues/562)) - ([d85427a](https://github.com/n0-computer/noq/commit/d85427adc1418132d392cb82c28ef86e1b86bfbb))
- *(proto)* Move `Connection::app_limited` to `PathData::app_limited` ([#580](https://github.com/n0-computer/noq/issues/580)) - ([712b231](https://github.com/n0-computer/noq/commit/712b231d6f7ece08b2709ad52cc184885e8f8a6c))
- *(proto)* Let server-side paths use recoverability hint on network change ([#579](https://github.com/n0-computer/noq/issues/579)) - ([f0ccfc2](https://github.com/n0-computer/noq/commit/f0ccfc2ece291d88d2d4185df644616dbb9840fe))
- *(proto)* Eventually stop resending PATH_CHALLENGEs on revalidations ([#590](https://github.com/n0-computer/noq/issues/590)) - ([e5e862c](https://github.com/n0-computer/noq/commit/e5e862cb76f0294750b69ce57ee8d6408b760458))
- *(proto)* Gracefully handle invalid PATH_ACK frames ([#592](https://github.com/n0-computer/noq/issues/592)) - ([a0481ac](https://github.com/n0-computer/noq/commit/a0481ac5a17003845bd3755318640b6abf994e6c))
- *(udp)* Propagate network-unreachable send errors instead of swallowing them ([#527](https://github.com/n0-computer/noq/issues/527)) - ([04eecd5](https://github.com/n0-computer/noq/commit/04eecd59a8b5886d560a34af8b54b64417da0561))
- Improve packet scheduling for not-validated paths ([#444](https://github.com/n0-computer/noq/issues/444)) - ([4bf3dc9](https://github.com/n0-computer/noq/commit/4bf3dc9bf2da77ff69d775d1ab4ca92fd24b77da))
- Don't track off-path probes in congestion control ([#532](https://github.com/n0-computer/noq/issues/532)) - ([39d2d91](https://github.com/n0-computer/noq/commit/39d2d91fac9d0bbe1c40ac5e08c6179a0c85f581))
- Revalidate existing paths during NAT traversal rounds ([#531](https://github.com/n0-computer/noq/issues/531)) - ([9c4ad62](https://github.com/n0-computer/noq/commit/9c4ad624c1e7350e77ca661cabaacb5b45fbf189))
- Ensure hole punching related frames don't get stuck ([#540](https://github.com/n0-computer/noq/issues/540)) - ([b236d7d](https://github.com/n0-computer/noq/commit/b236d7d89ae1f0318f761297131ff8b99f491e08))

### 🚜 Refactor

- *(clippy)* Enable manual_let_else lint and fix code ([#500](https://github.com/n0-computer/noq/issues/500)) - ([caf22ae](https://github.com/n0-computer/noq/commit/caf22ae6f8cabbf6ee305fc10b24b8cc743fb7cb))
- *(docs)* Describe PathData vs PacketNumberSpace ([#588](https://github.com/n0-computer/noq/issues/588)) - ([4b013e3](https://github.com/n0-computer/noq/commit/4b013e319b3cb6e598bf4a1fedc914a408da079f))
- *(proptests)* Improvements to runs and logging ([#495](https://github.com/n0-computer/noq/issues/495)) - ([d93c159](https://github.com/n0-computer/noq/commit/d93c159954c941c0900e64c95cdb5a08d161b37c))
- *(proptests)* Replace different proptests with a single proptest with a generated `PairSetup` ([#589](https://github.com/n0-computer/noq/issues/589)) - ([17ce182](https://github.com/n0-computer/noq/commit/17ce1822764a0e94f957a28cd388c49add99ce66))
- *(proto)* [**breaking**] Remove excessive `Arc`-wrapping ([#489](https://github.com/n0-computer/noq/issues/489)) - ([98507bd](https://github.com/n0-computer/noq/commit/98507bd0458c153edf9dada76108cf64d3c4ec9f))
- *(proto)* A few logging and comment tweaks ([#494](https://github.com/n0-computer/noq/issues/494)) - ([eab0948](https://github.com/n0-computer/noq/commit/eab0948804c8b3dbfd75fbb0ded656b5da2fdf9b))
- *(proto)* Switch from `Rng` trait to `CryptoRng` where appropriate ([#497](https://github.com/n0-computer/noq/issues/497)) - ([2a591fa](https://github.com/n0-computer/noq/commit/2a591fa0f4e11022461a79691433bd33a35c6c6a))
- *(proto)* Ordering of REACH_OUT and OBSERVED_ADDR building ([#508](https://github.com/n0-computer/noq/issues/508)) - ([f3a73c7](https://github.com/n0-computer/noq/commit/f3a73c7bdd722e9b18fbca0271845ccfa3412ea2))
- *(proto)* Rename path timers ([#517](https://github.com/n0-computer/noq/issues/517)) - ([21df564](https://github.com/n0-computer/noq/commit/21df56433ddc3a12ef2288ab062412e6d3661a4f))
- *(proto)* Remove support to bind a real socket in tests ([#537](https://github.com/n0-computer/noq/issues/537)) - ([27ef043](https://github.com/n0-computer/noq/commit/27ef0435f799d19a81101476d5cf8845cc076d72))
- *(proto)* Move FrameStats into PathStats ([#521](https://github.com/n0-computer/noq/issues/521)) - ([fd8e5ba](https://github.com/n0-computer/noq/commit/fd8e5bafe04788bbc969302ad65ce36ed02501ef))
- *(proto)* Rename `maybe_queue_probe` to `queue_tail_loss_probe` ([#558](https://github.com/n0-computer/noq/issues/558)) - ([a2245b5](https://github.com/n0-computer/noq/commit/a2245b5f119b674348be608ab64898f49c281ecb))
- *(proto)* Only emit handshake confirmed event for the first `HandshakeDone` frame ([#566](https://github.com/n0-computer/noq/issues/566)) - ([44387fd](https://github.com/n0-computer/noq/commit/44387fda1bccf2872e1961f3f44561ff07dcefae))
- *(proto)* Spell out packet number ([#577](https://github.com/n0-computer/noq/issues/577)) - ([e377e95](https://github.com/n0-computer/noq/commit/e377e9539d65c3446f583900427e777b81ef2642))
- Remove redundant log message ([#498](https://github.com/n0-computer/noq/issues/498)) - ([9725dfb](https://github.com/n0-computer/noq/commit/9725dfb2706d11bd5908e57696a62661456e705e))
- Remove poll_read_buf from public api ([#548](https://github.com/n0-computer/noq/issues/548)) - ([c9d9bf3](https://github.com/n0-computer/noq/commit/c9d9bf35092d9291e0917b387141979bfeff88c6))

### 📚 Documentation

- Create CHANGELOG.md - ([87bc101](https://github.com/n0-computer/noq/commit/87bc1018474de88bfffc6c98a5b76f2dc9376e22))

### 🧪 Testing

- *(proto)* Set an idle timeout for paths in multipath proptests ([#538](https://github.com/n0-computer/noq/issues/538)) - ([0d24883](https://github.com/n0-computer/noq/commit/0d24883330074569209a143d1064b28a65c3986f))
- *(proto)* Fix packet direction log line ([#543](https://github.com/n0-computer/noq/issues/543)) - ([bf31f07](https://github.com/n0-computer/noq/commit/bf31f0785d9fa8bfde5f64640d6cdb1bc8c585e9))
- *(proto)* Add test for sending path abandon on path itself ([#549](https://github.com/n0-computer/noq/issues/549)) - ([2e0ba5a](https://github.com/n0-computer/noq/commit/2e0ba5a5c83e47a820fbb94895beb41940c0861e))
- *(proto)* Test closing while the client migrates ([#552](https://github.com/n0-computer/noq/issues/552)) - ([c4fa721](https://github.com/n0-computer/noq/commit/c4fa7212e923179b5968f65583526f008df2dd41))
- *(proto)* Stop driving the connection before a path idle timer fires ([#557](https://github.com/n0-computer/noq/issues/557)) - ([b3e07e9](https://github.com/n0-computer/noq/commit/b3e07e92ba3d13fa1f91f23c8f500a02659e2056))

### ⚙️ Miscellaneous Tasks

- *(ci)* Add standard text to beta ci notifications ([#506](https://github.com/n0-computer/noq/issues/506)) - ([7e9f628](https://github.com/n0-computer/noq/commit/7e9f628c47edb3f909f4a63ee3bed41bf7ed464e))
- *(docs)* Check internal docs as well ([#499](https://github.com/n0-computer/noq/issues/499)) - ([a92084b](https://github.com/n0-computer/noq/commit/a92084ba7a90a218b92716f6418e65b8928a1bd7))
- *(proto)* Update to rand 0.10 ([#511](https://github.com/n0-computer/noq/issues/511)) - ([1280ffd](https://github.com/n0-computer/noq/commit/1280ffd01bb1f031a61a5e8ea1a09f4e0baed466))
- *(tests)* Rename proptests modules to all be consistent ([#493](https://github.com/n0-computer/noq/issues/493)) - ([2b04ddf](https://github.com/n0-computer/noq/commit/2b04ddf5c825c04ebf4fbbd346bde3a6abe5e06f))
- Fix release config - ([6b17679](https://github.com/n0-computer/noq/commit/6b1767964daab5b8bc26933f222819612339fa4b))
- Build on RISC-V espidf to check that we properly use portable_atomic and don't use advanced posix features. ([#492](https://github.com/n0-computer/noq/issues/492)) - ([9234776](https://github.com/n0-computer/noq/commit/9234776e6448d7925dd02f508ff175f16d7d4339))
- Fix CID exhaustion check overflow on 32-bit targets ([#564](https://github.com/n0-computer/noq/issues/564)) - ([93f4216](https://github.com/n0-computer/noq/commit/93f421632b9bc61525e1d7559d51d40d77cc5149))
- Add more cargo-make targets and update CI template ([#586](https://github.com/n0-computer/noq/issues/586)) - ([de242d8](https://github.com/n0-computer/noq/commit/de242d85f48178d411afa9160f101da30b2adef7))
- Fix release config - ([bbcae8e](https://github.com/n0-computer/noq/commit/bbcae8e0ab0d7747b770ba4fd633589ba0173574))

### Bench

- Allow configuring the runtime type in the bulk bench ([#280](https://github.com/n0-computer/noq/issues/280)) - ([2156ac1](https://github.com/n0-computer/noq/commit/2156ac1c3034b3d5681c1e35c4d687d43704b272))

### Deps

- *(proto)* Don't require ring in wasm builds ([#503](https://github.com/n0-computer/noq/issues/503)) - ([8adda44](https://github.com/n0-computer/noq/commit/8adda44486fdf33f08ae7481b2c3b3a4db587548))
- Hide the tokio streams behind simple newtype wrappers ([#547](https://github.com/n0-computer/noq/issues/547)) - ([b212bbc](https://github.com/n0-computer/noq/commit/b212bbcaccaa82089cc17fb29c4458d113a0cae6))

### Proto

- *(tests)* Document the routing table for proptests a bit ([#490](https://github.com/n0-computer/noq/issues/490)) - ([39510cd](https://github.com/n0-computer/noq/commit/39510cd4431206fdf6827b3807b95debf03d6d7a))

## [0.17.0](https://github.com/n0-computer/noq/compare/iroh-quinn-v0.16.1..v0.17.0) - 2026-03-09

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

## [iroh-quinn-v0.16.1](https://github.com/n0-computer/noq/compare/iroh-quinn-proto-v0.15.1..iroh-quinn-v0.16.1) - 2026-01-31

### ⚙️ Miscellaneous Tasks

- Release - ([4695fbf](https://github.com/n0-computer/noq/commit/4695fbfe1f53eb0466341744fb604da896db66ab))

## [iroh-quinn-proto-v0.15.1](https://github.com/n0-computer/noq/compare/iroh-quinn-v0.16.0..iroh-quinn-proto-v0.15.1) - 2026-01-31

### 🐛 Bug Fixes

- *(proto)* Avoid panicking in `Connection::pto_max_path` ([#366](https://github.com/n0-computer/noq/issues/366)) - ([5660b3e](https://github.com/n0-computer/noq/commit/5660b3e8afa69b60cd177e4d579795c01753189f))
- *(proto)* Send close immediately ([#385](https://github.com/n0-computer/noq/issues/385)) - ([3e729dc](https://github.com/n0-computer/noq/commit/3e729dc3926e8bac7b094fcd3562706133c02400))

### 🧪 Testing

- *(proto)* Simulate passive migrations in proptests ([#359](https://github.com/n0-computer/noq/issues/359)) - ([cc9682a](https://github.com/n0-computer/noq/commit/cc9682a39330e4eee8711609757eb757019d5174))

### ⚙️ Miscellaneous Tasks

- Disable solaris tests in daily CI as they hang ([#377](https://github.com/n0-computer/noq/issues/377)) - ([25f35c2](https://github.com/n0-computer/noq/commit/25f35c2fe3242cae6a1e4dd9f1810cc0f893eada))
- Release - ([d1c70a8](https://github.com/n0-computer/noq/commit/d1c70a87d095e5ba52bacc0820071d7a972b67e8))

## [iroh-quinn-v0.16.0](https://github.com/n0-computer/noq/compare/iroh-quinn-proto-v0.15.0..iroh-quinn-v0.16.0) - 2026-01-28

### ⚙️ Miscellaneous Tasks

- Release - ([e416725](https://github.com/n0-computer/noq/commit/e41672537db640c964f61a2597fce0d474366c10))

## [iroh-quinn-proto-v0.15.0](https://github.com/n0-computer/noq/compare/iroh-quinn-v0.15.0..iroh-quinn-proto-v0.15.0) - 2026-01-28

### ⛰️  Features

- *(ci)* Add performance benchmarks comparing iroh-quinn vs upstream - ([75e39be](https://github.com/n0-computer/noq/commit/75e39be3760ecf6a99c623d399dbf9f2f6295a9c))
- *(ci)* Add performance benchmarks comparing iroh-quinn vs upstream ([#299](https://github.com/n0-computer/noq/issues/299)) - ([43a7a7c](https://github.com/n0-computer/noq/commit/43a7a7c089088472298380d97497043a2967dee3))
- *(proto)* [**breaking**] Update to final IANA numbers for multipath ([#302](https://github.com/n0-computer/noq/issues/302)) - ([2ae68e6](https://github.com/n0-computer/noq/commit/2ae68e6dd190575ab902ec5af4bc2069df27b017))
- *(proto)* Use `UdpStats` in `PathStats` and add receive stats ([#332](https://github.com/n0-computer/noq/issues/332)) - ([be298f3](https://github.com/n0-computer/noq/commit/be298f39adbfd5e6f7c26341c2b870399048f139))

### 🐛 Bug Fixes

- *(multipath)* Handle missing remote CIDs better ([#336](https://github.com/n0-computer/noq/issues/336)) - ([3752420](https://github.com/n0-computer/noq/commit/37524202b6fcd598a94d02ea9009a97f1347b4f0))
- *(proto)* Store pending PATH_CID_BLOCKED frames in a BTreeSet ([#307](https://github.com/n0-computer/noq/issues/307)) - ([4b6ac1c](https://github.com/n0-computer/noq/commit/4b6ac1c7daacf71ca70726a45cc04e6d4734ac12))
- *(proto)* Use path PTOs for PATH_ABANDON calculations ([#291](https://github.com/n0-computer/noq/issues/291)) - ([ee98007](https://github.com/n0-computer/noq/commit/ee980072e0ab795e1c803c8e88111c7a8026a0a4))
- *(proto)* Check `abandoned_paths` in `open_path_ensure` ([#319](https://github.com/n0-computer/noq/issues/319)) - ([4de9876](https://github.com/n0-computer/noq/commit/4de9876c537f9afdd93af8f7ae6d32bf1924ec5c))
- *(proto)* Some frames were sent on non-validated path ([#330](https://github.com/n0-computer/noq/issues/330)) - ([6177c8d](https://github.com/n0-computer/noq/commit/6177c8d05ca198b2069bf478fecdf14450c30309))
- *(proto)* Properly encode can_coalesce during packet building ([#334](https://github.com/n0-computer/noq/issues/334)) - ([19274cf](https://github.com/n0-computer/noq/commit/19274cf4d5717c811e61d4d2fbebac71e7c4ad0f))
- *(proto)* Don't close paths from previous HP round we're still interested in ([#341](https://github.com/n0-computer/noq/issues/341)) - ([ce4f347](https://github.com/n0-computer/noq/commit/ce4f3470322ad01a5e463e054ec25a199413acc9))
- *(proto)* Stop all path timers when closing it ([#350](https://github.com/n0-computer/noq/issues/350)) - ([1a2bd11](https://github.com/n0-computer/noq/commit/1a2bd1186b8d6269b5992b2effd9d3ac1420dffb))
- *(quinn-proto)* Correctly round-trip `ConnectionClose` encoding. ([#304](https://github.com/n0-computer/noq/issues/304)) - ([c31d4a6](https://github.com/n0-computer/noq/commit/c31d4a64f59dfd88240abb11e1a3f85af9189231))
- *(quinn-proto)* Do not reuse buffers across network paths ([#339](https://github.com/n0-computer/noq/issues/339)) - ([a772fbe](https://github.com/n0-computer/noq/commit/a772fbe0950461a6a0e189a97a5bdfaa6592c73d))
- *(windows)* Use `effective_segment_size` - ([d1bbfc0](https://github.com/n0-computer/noq/commit/d1bbfc044f662671f8ef2447261b943afad1ba59))
- Fix condition to generate PROTOCOL_VIOLATION ([#301](https://github.com/n0-computer/noq/issues/301)) - ([fbf225e](https://github.com/n0-computer/noq/commit/fbf225e0958cb4b9609f75a9accb6bf004fc5ed4))
- Use better terms in logging ([#343](https://github.com/n0-computer/noq/issues/343)) - ([7a8aeb9](https://github.com/n0-computer/noq/commit/7a8aeb9609c435e1a97882ecadbe785f3e90bd3f))
- Emit PathNewConnectionId frame in qlog ([#363](https://github.com/n0-computer/noq/issues/363)) - ([cd7a9d8](https://github.com/n0-computer/noq/commit/cd7a9d84430f11e2e8b56304f385c185a4f1ad82))
- Expect missing CIDs when migrating ([#362](https://github.com/n0-computer/noq/issues/362)) - ([ab03f16](https://github.com/n0-computer/noq/commit/ab03f1603890adc96ea3421aeb7365bddd305eb4))
- Do not send off-path data when closing ([#358](https://github.com/n0-computer/noq/issues/358)) - ([aa6c373](https://github.com/n0-computer/noq/commit/aa6c3735cee6d3074d5421a97b5e48332de8e7b3))
- Do not send CONNECTION_CLOSE to unvalidated remotes ([#361](https://github.com/n0-computer/noq/issues/361)) - ([d7e98af](https://github.com/n0-computer/noq/commit/d7e98af8cf261987450cc78a8b8747eca2745dc0))

### 🚜 Refactor

- *(proto)* Store ArrayRangeSet in Acks ([#320](https://github.com/n0-computer/noq/issues/320)) - ([b5deb22](https://github.com/n0-computer/noq/commit/b5deb223d6860f268a9d406169ce4646e0d39b0b))
- *(proto)* Cleanup Connection::space_can_send ([#333](https://github.com/n0-computer/noq/issues/333)) - ([22169a5](https://github.com/n0-computer/noq/commit/22169a5a0283a91cd78afda775aa0bf87dd1da30))
- *(proto)* Extract address canonicalization and local socket family detection into fns ([#346](https://github.com/n0-computer/noq/issues/346)) - ([c38be74](https://github.com/n0-computer/noq/commit/c38be740eb4183a11d3dbbbca0b86d48a1f72ebb))
- *(quinn-proto)* Move frame sending tasks to PacketBuilder ([#295](https://github.com/n0-computer/noq/issues/295)) - ([7663451](https://github.com/n0-computer/noq/commit/7663451642bffe9077428c6f7e1cb5c3b4f4da97))
- *(quinn-proto)* Misc code improvements from pr reviews ([#327](https://github.com/n0-computer/noq/issues/327)) - ([ddbdf53](https://github.com/n0-computer/noq/commit/ddbdf535a8864289c5d07c6083d83c5512e8f531))
- *(quinn-udp)* Make `decode_recv` generic over `MsgHdr` - ([d22db52](https://github.com/n0-computer/noq/commit/d22db5291ca69c00852b0827b42eb00a8fe4a46d))
- Introduce `Transmit::effective_segment_size` - ([9008656](https://github.com/n0-computer/noq/commit/9008656e7f9e8c2e110a2ee2be2f88f03b4d25a6))
- Some logging consistency ([#331](https://github.com/n0-computer/noq/issues/331)) - ([bd88e2d](https://github.com/n0-computer/noq/commit/bd88e2d0dc9ecc9a5714e045bfd6a925173d13da))
- Update to rust edition 2024 ([#328](https://github.com/n0-computer/noq/issues/328)) - ([5f19d75](https://github.com/n0-computer/noq/commit/5f19d75275d9342b0092e70cb6976d362708c233))
- Rename `rem_cid` to `remote_cid` and friends ([#351](https://github.com/n0-computer/noq/issues/351)) - ([7706e5a](https://github.com/n0-computer/noq/commit/7706e5a4e1972fbd3712cbf28d65518e90b215c5))
- Some much needed renaming of variables mostly ([#354](https://github.com/n0-computer/noq/issues/354)) - ([e6c004e](https://github.com/n0-computer/noq/commit/e6c004e1445883204d75949ca9a7b5bcbf7a0015))
- Log which path is lacking CIDs ([#355](https://github.com/n0-computer/noq/issues/355)) - ([11b684d](https://github.com/n0-computer/noq/commit/11b684ded333b0cc6daefac97ce52102c9fa2618))
- Split poll_transmit into per-path and per-space parts ([#338](https://github.com/n0-computer/noq/issues/338)) - ([2cc515a](https://github.com/n0-computer/noq/commit/2cc515adbc1215d40d9916ad2b27546d6df6c3f3))

### 📚 Documentation

- *(proto)* Improve path generations internal docs ([#311](https://github.com/n0-computer/noq/issues/311)) - ([daaa4e8](https://github.com/n0-computer/noq/commit/daaa4e84f838c56c880d785cf396566166b287dc))

### 🧪 Testing

- Add proptest arbitraries for various quinn-proto types and implement encoder roundtrip tests. ([#325](https://github.com/n0-computer/noq/issues/325)) - ([1df7657](https://github.com/n0-computer/noq/commit/1df76577f753afd86f88cd949b434d94a6255fb8))
- Complete encoder proptests ([#340](https://github.com/n0-computer/noq/issues/340)) - ([9f4c68e](https://github.com/n0-computer/noq/commit/9f4c68ed1775e637428a39e7bcaa2a0fc3fc61d1))
- Skip proptests in the default runs ([#356](https://github.com/n0-computer/noq/issues/356)) - ([e424c7f](https://github.com/n0-computer/noq/commit/e424c7fb789b73d7f82b5cc12dd09366c35a0f35))

### ⚙️ Miscellaneous Tasks

- *(quinn-udp)* Use consts from libc - ([0a6c94a](https://github.com/n0-computer/noq/commit/0a6c94af8c881aad7f798968bd2f89cf1405b493))
- Fix `unnecessary_unwrap` clippy - ([f4b3a21](https://github.com/n0-computer/noq/commit/f4b3a2114976dd6a59e85473f3946997bd1f4d06))
- Merge upstream until b53828f0e48752f771c2538364c8876a907a0345 ([#303](https://github.com/n0-computer/noq/issues/303)) - ([0c3de90](https://github.com/n0-computer/noq/commit/0c3de90490cdc385560dfed2c6e4ce1dd02f223d))
- Unblock CI ([#321](https://github.com/n0-computer/noq/issues/321)) - ([39f776d](https://github.com/n0-computer/noq/commit/39f776db5b02a4b4ff4c39315ef6595b0bca58ef))
- Revert sccache removal ([#326](https://github.com/n0-computer/noq/issues/326)) - ([afc9ed4](https://github.com/n0-computer/noq/commit/afc9ed49d22566419e5d75b677771bca64ee64c1))
- Add CI to ensure docs-rs publishing will work ([#360](https://github.com/n0-computer/noq/issues/360)) - ([bf4c648](https://github.com/n0-computer/noq/commit/bf4c64874df80f0d4feee4b0c7d9d82d9f6e9b21))
- Release - ([086cf76](https://github.com/n0-computer/noq/commit/086cf769b339d9fce4bfa3c60edde6b47586cbb5))
- Release - ([349e829](https://github.com/n0-computer/noq/commit/349e82913aecd55948cb429454ddc2ded57e83d3))

### Quinn

- Re-export InvalidCid from quinn-proto - ([cac692c](https://github.com/n0-computer/noq/commit/cac692cdf503f51bf39e0470c7fe76dcc6fbcea2))

## [iroh-quinn-v0.15.0](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.14..iroh-quinn-v0.15.0) - 2026-01-09

### ⛰️  Features

- *(proto)* Expand qlog support ([#181](https://github.com/n0-computer/noq/issues/181)) - ([2380578](https://github.com/n0-computer/noq/commit/238057833af99ad7ae123dd9144436ee37a2d1e7))
- *(proto)* Improve PTO calculations on shutdown - ([0cfc00d](https://github.com/n0-computer/noq/commit/0cfc00d393037baf976b610af014d6014e797cad))
- *(proto)* Improve PTO calculations on shutdown ([#269](https://github.com/n0-computer/noq/issues/269)) - ([4dcb3a7](https://github.com/n0-computer/noq/commit/4dcb3a7f05d1b3eb5441334ed301fed316fd79aa))
- *(quinn)* Expose per path ping method - ([6861f4f](https://github.com/n0-computer/noq/commit/6861f4f0b2d4b28db5a6d44cbd3d3708cf1725d7))
- *(quinn)* Expose per path ping method ([#290](https://github.com/n0-computer/noq/issues/290)) - ([200a68a](https://github.com/n0-computer/noq/commit/200a68a0d92a6a3d6392ba229429bee7c0eb5b44))
- *(quinn-proto)* Use RTT from path challenges, implement path challenge resending ([#178](https://github.com/n0-computer/noq/issues/178)) - ([1fdbf69](https://github.com/n0-computer/noq/commit/1fdbf6912e03379f5a78bd7db602eecd54a10c65))
- *(quinn-proto)* Retry client sent probes when the failures might be spurious ([#271](https://github.com/n0-computer/noq/issues/271)) - ([c8f5510](https://github.com/n0-computer/noq/commit/c8f55101b5b90eb01c0d54c9f6f2f12f338caeef))
- *(test)* Set `dst_ip` for incoming datagrams in `TestEndpoint` ([#248](https://github.com/n0-computer/noq/issues/248)) - ([ad4c99f](https://github.com/n0-computer/noq/commit/ad4c99f126c9e0de3f331cf1eb645d1d305e2b5b))
- Update multipath to latest released quinn (b2b930a) ([#128](https://github.com/n0-computer/noq/issues/128)) - ([81734b9](https://github.com/n0-computer/noq/commit/81734b917552e719ed4c2f9bd01e77b7999e041c))
- Add `Connection::on_close` to get notified on close without keeping the connection alive ([#153](https://github.com/n0-computer/noq/issues/153)) - ([19817fc](https://github.com/n0-computer/noq/commit/19817fcc9b9f9a14473e2724262218aa59e42c69))
- Add n0's version of quic nat hole punching ([#177](https://github.com/n0-computer/noq/issues/177)) - ([fc187cb](https://github.com/n0-computer/noq/commit/fc187cb1511e3905f8823ebe898cd101f4ce1d5e))
- Expose feature fast-apple-datapath ([#204](https://github.com/n0-computer/noq/issues/204)) - ([ea9dbb9](https://github.com/n0-computer/noq/commit/ea9dbb9fb098438f69f7e72fada0512fe41956df))
- Make `TransportConfig` Clone ([#213](https://github.com/n0-computer/noq/issues/213)) - ([9c66e79](https://github.com/n0-computer/noq/commit/9c66e79377e6d75f34e45b5a02c5dbb4f1633351))
- [**breaking**] Update qlog to latest drafts, add more events and QUIC extensions, emit traces per connection ([#208](https://github.com/n0-computer/noq/issues/208)) - ([abbe27b](https://github.com/n0-computer/noq/commit/abbe27bf0738a6d8c31f6971e2c926818d594d71))
- Implement `CidQueue::next_reserved` ([#180](https://github.com/n0-computer/noq/issues/180)) - ([b49b43f](https://github.com/n0-computer/noq/commit/b49b43f2cc3b8ab12e0d6ac4215a6bbbbfa019ac))
- Introduce UnorderedRecvStream newtype ([#257](https://github.com/n0-computer/noq/issues/257)) - ([8dfdc04](https://github.com/n0-computer/noq/commit/8dfdc04e138a310e95489f714bc05d0d38a44379))
- Publically export `UnorderedRecvStream` ([#260](https://github.com/n0-computer/noq/issues/260)) - ([18604d3](https://github.com/n0-computer/noq/commit/18604d36beb6d83ff86dc3354a453e56da3082b0))

### 🐛 Bug Fixes

- *(ci)* Daily mac os job ([#229](https://github.com/n0-computer/noq/issues/229)) - ([11a024b](https://github.com/n0-computer/noq/commit/11a024ba733c8eb00a5cb09b61a649127b52463d))
- *(ci)* Windows deps - ([eb59f64](https://github.com/n0-computer/noq/commit/eb59f64ae57331968a1f506d78887c337484a048))
- *(ci)* Windows deps ([#267](https://github.com/n0-computer/noq/issues/267)) - ([d1a6b5a](https://github.com/n0-computer/noq/commit/d1a6b5a3d174b4bf56e9cb7a5026ba0097f70c3a))
- *(ci)* Windows test build fix - ([c4fb84b](https://github.com/n0-computer/noq/commit/c4fb84b8c4c7fa5c0c325a4288a020290ef22f23))
- *(ci)* Windows test build fix ([#273](https://github.com/n0-computer/noq/issues/273)) - ([9947dfa](https://github.com/n0-computer/noq/commit/9947dfa162dbfb20759fc96411ee599173b53391))
- *(ci)* Follow discord notification conventions - ([4d740d6](https://github.com/n0-computer/noq/commit/4d740d64ec62f8c9fff95a3e49a6c5cd9712ace4))
- *(docs)* Fix a number of doc links - ([3da4850](https://github.com/n0-computer/noq/commit/3da4850661cad3fb0332e022ac80c315e8d0b430))
- *(docs)* Fix a number of doc links ([#285](https://github.com/n0-computer/noq/issues/285)) - ([d840435](https://github.com/n0-computer/noq/commit/d840435447d4947e44ad44b55e0be6a1094f39cc))
- *(proto)* Allow closed -> closed transitions - ([17e43bf](https://github.com/n0-computer/noq/commit/17e43bf45e2e9df38f631d9e7eeb9c8a09b2625a))
- *(proto)* Do not assume PN for the crypto packet - ([5f466df](https://github.com/n0-computer/noq/commit/5f466df38fe444117bfdf95c350ac807cdebb0b4))
- *(proto)* Do not assume PN for the crypto packet ([#188](https://github.com/n0-computer/noq/issues/188)) - ([8f78a67](https://github.com/n0-computer/noq/commit/8f78a67c9baa05d276077a5a975103f6a6ebb8c8))
- *(proto)* Correct logic change for noting congestion on acks - ([21a8fdc](https://github.com/n0-computer/noq/commit/21a8fdc68f5e72f354cac0222513c2f0b3e67ad1))
- *(proto)* Correctly check `space.rx_packet` in key updates ([#195](https://github.com/n0-computer/noq/issues/195)) - ([3d7621f](https://github.com/n0-computer/noq/commit/3d7621f04c0f03405464c1c1217afc43f68fb80c))
- *(proto)* Send path acks only in 1RTT space ([#220](https://github.com/n0-computer/noq/issues/220)) - ([33156eb](https://github.com/n0-computer/noq/commit/33156eb69a4220b3ba0b3a25ba4bcbce533b5dbb))
- *(proto)* Don't try to clean up ACKs on packet number spaces of forgotten paths ([#230](https://github.com/n0-computer/noq/issues/230)) - ([d4d11a1](https://github.com/n0-computer/noq/commit/d4d11a153892720ceeb2db407f93591042ed6323))
- *(proto)* Handle new CIDs for closed paths ([#224](https://github.com/n0-computer/noq/issues/224)) - ([704a3f6](https://github.com/n0-computer/noq/commit/704a3f613ef87a5ed93152c0da8c4b754d2dfbd7))
- *(proto)* Ensure ImmediateAcks are sent in Data space ([#226](https://github.com/n0-computer/noq/issues/226)) - ([67df91c](https://github.com/n0-computer/noq/commit/67df91c6670cc48ff2043563e4da388541b4e20b))
- *(proto)* Emit updated qlog tuple when a path's remote changes ([#236](https://github.com/n0-computer/noq/issues/236)) - ([2e1b164](https://github.com/n0-computer/noq/commit/2e1b164fdef3e7b7064bd0a7349bee7f73cca53f))
- *(proto)* Return error in initiate_nat_traversal_round ([#254](https://github.com/n0-computer/noq/issues/254)) - ([36fe698](https://github.com/n0-computer/noq/commit/36fe698a36efd4766a3b5cd1bacf5eb353177916))
- *(proto)* Ensure errors are returned from `proto::Connection::poll`, fix PATH_ABANDON logic ([#234](https://github.com/n0-computer/noq/issues/234)) - ([4f9d519](https://github.com/n0-computer/noq/commit/4f9d519ef4a299b5b36629b2a8f41b8888f41b8d))
- *(proto)* Don't assume a path is validated if we had another path on the same remote ([#258](https://github.com/n0-computer/noq/issues/258)) - ([9cac306](https://github.com/n0-computer/noq/commit/9cac306da1f0acba46322feebedc62467babd0c9))
- *(proto)* Do not attempt to initate NAT traversal on a closed connection - ([1ac2c3c](https://github.com/n0-computer/noq/commit/1ac2c3c9145d05f07e995aef1f7293a0693283aa))
- *(proto)* Do not attempt to initate NAT traversal on a closed connection ([#268](https://github.com/n0-computer/noq/issues/268)) - ([7a3f33e](https://github.com/n0-computer/noq/commit/7a3f33ecc17102cc623c0f0ed13222f81b81b9fb))
- *(proto)* Send the close frame on the available path - ([cfd7bbe](https://github.com/n0-computer/noq/commit/cfd7bbebeb6b71fdf955b40fb46782c2843f0dd1))
- *(proto)* Send the close frame on the available path ([#274](https://github.com/n0-computer/noq/issues/274)) - ([9d73ec4](https://github.com/n0-computer/noq/commit/9d73ec4a9476132cb0d38b09e63e2ceb85670b5c))
- *(proto)* Actually use per path default configs - ([6dbfb68](https://github.com/n0-computer/noq/commit/6dbfb684e5a85e2900185debff579aaa8c5b23a8))
- *(proto)* Actually use per path default configs ([#286](https://github.com/n0-computer/noq/issues/286)) - ([e8953c6](https://github.com/n0-computer/noq/commit/e8953c6f285cef31bb8cef2902eb2f7d7d8ae8ca))
- *(quinn-proto)* Set the hole punching round - ([56294fe](https://github.com/n0-computer/noq/commit/56294fe70799adf81b4144c11662e3c943723782))
- *(quinn-proto)* Prevent O(N^2) behaviour when sending a lot of small values in SendBuffer ([#247](https://github.com/n0-computer/noq/issues/247)) - ([ad47fc6](https://github.com/n0-computer/noq/commit/ad47fc662163f8bb58b4552e4b2db9aeac923041))
- *(udp/unix)* Don't panic on unknown address family - ([c1c5686](https://github.com/n0-computer/noq/commit/c1c56861cff978bcedddc4c4cd953eca1d1ef5b2))
- Key update on the first packet of a new PathId ([#165](https://github.com/n0-computer/noq/issues/165)) - ([34eee26](https://github.com/n0-computer/noq/commit/34eee26fc7fbc9ccb140380c0219ee387e70328d))
- Reset tail-loss probes when discarding a space ([#171](https://github.com/n0-computer/noq/issues/171)) - ([3b3a5e8](https://github.com/n0-computer/noq/commit/3b3a5e8ec6f68cf8242b182f1ae58a6d65aa9449))
- Accept or reject incoming frames based on `self.local_max_path_id` instead of `Connection::max_path_id` ([#183](https://github.com/n0-computer/noq/issues/183)) - ([5bd31b7](https://github.com/n0-computer/noq/commit/5bd31b72f83170c4c93bee8b0f667da521a2c1eb))
- Ignore PATH_ACKs on abandoned paths - ([4fcc804](https://github.com/n0-computer/noq/commit/4fcc804f9775027485f84884c869d13749594d34))
- Ignore PATH_ACKs on abandoned paths ([#192](https://github.com/n0-computer/noq/issues/192)) - ([d512332](https://github.com/n0-computer/noq/commit/d5123326ceb7e4044b922a234930dcb408e1d11c))
- Don't allow sending path status frames on non-multipath connections - ([ca1b4ff](https://github.com/n0-computer/noq/commit/ca1b4ffbcf466d9c8096d49c21e38a00a442c4e7))
- Don't allow sending path status frames on non-multipath connections ([#191](https://github.com/n0-computer/noq/issues/191)) - ([1b83e4f](https://github.com/n0-computer/noq/commit/1b83e4fea7fe07e021d8560f1fa99770bdd77671))
- Fmt & spelling - ([2f508d5](https://github.com/n0-computer/noq/commit/2f508d5ab2cf7c0bebc346babb26435349b0876c))
- Disable GSO on windows if failures happen ([#209](https://github.com/n0-computer/noq/issues/209)) - ([c0cf854](https://github.com/n0-computer/noq/commit/c0cf8540ebd74d09711e3d270152f2a439376e12))
- Add ci timeout to codecov ([#217](https://github.com/n0-computer/noq/issues/217)) - ([f95435e](https://github.com/n0-computer/noq/commit/f95435ef10a51883c4bd777e8910899e93666758))
- Run MTU discovery on all paths ([#215](https://github.com/n0-computer/noq/issues/215)) - ([685f995](https://github.com/n0-computer/noq/commit/685f995826df0aea586d6b596fb861e2588d75a8))
- Do not use Instant::now in proto tests ([#223](https://github.com/n0-computer/noq/issues/223)) - ([29c186d](https://github.com/n0-computer/noq/commit/29c186d154278560a4968532fbfe487e37ca226d))
- Cleanup path closing errors ([#221](https://github.com/n0-computer/noq/issues/221)) - ([988efd8](https://github.com/n0-computer/noq/commit/988efd8578a5ae429f56cf237e59243c4213b412))
- [**breaking**] Use PathId in more APIs and remove Connection::network_path_changed ([#210](https://github.com/n0-computer/noq/issues/210)) - ([79aa0f8](https://github.com/n0-computer/noq/commit/79aa0f8092ecc2d7df43d5f750960fb7675632f6))
- Add missing frames to qlog ([#228](https://github.com/n0-computer/noq/issues/228)) - ([b29730b](https://github.com/n0-computer/noq/commit/b29730b70fe2afac55958c6aee7901177bfc5a03))
- Check higher spaces if blocked for any reason ([#232](https://github.com/n0-computer/noq/issues/232)) - ([46a3fe9](https://github.com/n0-computer/noq/commit/46a3fe9976a71af38304ab15ac5f273e8d36ae78))
- Only use PATH_CHALLENGE/RESPONSE RTT for initial RTT ([#235](https://github.com/n0-computer/noq/issues/235)) - ([2cce29c](https://github.com/n0-computer/noq/commit/2cce29ce14adada82249c7b7e7c0c6b21aa8a07f))
- Improve condition of when a PATH_RESPONSE RTT can be used ([#237](https://github.com/n0-computer/noq/issues/237)) - ([df0828a](https://github.com/n0-computer/noq/commit/df0828a7b0f4b1738d82b262557a877a99ac6a25))
- Path cid panic ([#242](https://github.com/n0-computer/noq/issues/242)) - ([2cf15d2](https://github.com/n0-computer/noq/commit/2cf15d246caa6a14add0405d50d55289755d8699))
- Include off-path challenges in setting challenge lost timer ([#261](https://github.com/n0-computer/noq/issues/261)) - ([caccc67](https://github.com/n0-computer/noq/commit/caccc6754dee8fc62be968263b8aa965822f4f91))
- Stop PathChallengeLost timer when path opening timer fires - ([c486b7c](https://github.com/n0-computer/noq/commit/c486b7c9eccbbf3d5af2f3ad5a19279964983768))
- Always double deref Connection -> ConnectionInner - ([2953626](https://github.com/n0-computer/noq/commit/29536261f66a18330abeaea37b54afac4e184f35))
- Always double deref Connection -> ConnectionInner ([#277](https://github.com/n0-computer/noq/issues/277)) - ([51907c8](https://github.com/n0-computer/noq/commit/51907c821a1bb24adef5dcc8aee9620ddbc7d970))

### 🚜 Refactor

- *(proto)* Box AcceptError - ([7d7afb5](https://github.com/n0-computer/noq/commit/7d7afb5b3e4b4b3475cf5ffe3d529f1fe8a9e113))
- *(quinn-proto)* Make `FrameType` a proper enum ([#292](https://github.com/n0-computer/noq/issues/292)) - ([77d9d1e](https://github.com/n0-computer/noq/commit/77d9d1ebecf73e3de43c0cdccc0f5bf0e37871e0))
- *(quinn-proto)* Split the coding::Codec ([#293](https://github.com/n0-computer/noq/issues/293)) - ([1b96a5f](https://github.com/n0-computer/noq/commit/1b96a5f819e1b6b2c3d3f2f462ffa5b1ee3fff50))
- *(quinn-proto)* Create an encodable type for non encodable frames ([#294](https://github.com/n0-computer/noq/issues/294)) - ([226ff0d](https://github.com/n0-computer/noq/commit/226ff0de42ebdbfd78c965046149e1c6e51b6048))
- *(quinn-proto)* Use enum-assoc instead of manual macro for frametype - ([bacde87](https://github.com/n0-computer/noq/commit/bacde87c7195ec1a2428500cde16810371d55b01))
- *(quinn-proto)* Use enum-assoc instead of manual macro for FrameType ([#298](https://github.com/n0-computer/noq/issues/298)) - ([7ce0396](https://github.com/n0-computer/noq/commit/7ce0396198789ff7b66b5d71bfc12fbd162589df))
- Split path stats from connection stats - ([edb72f9](https://github.com/n0-computer/noq/commit/edb72f9e7ca6c606472836d4e7d4c2dfdf0bdd00))
- Split path stats from connection stats ([#168](https://github.com/n0-computer/noq/issues/168)) - ([dced59e](https://github.com/n0-computer/noq/commit/dced59ee0ae406fa4445136c33c598448e0e738f))
- Use constants for error codes - ([61c87ab](https://github.com/n0-computer/noq/commit/61c87aba64021f314d6252fc9a687a253c70a04e))
- Improve error storage for closed connections  - ([ca37573](https://github.com/n0-computer/noq/commit/ca37573e5bc634e6bd3e453e02fc95dbb569306f))
- [**breaking**] Rename log feature to tracing-log - ([377af28](https://github.com/n0-computer/noq/commit/377af28843a8e05e6c2d73798f1e9cbc30796531))
- Pass ConnectionId by value - ([1350132](https://github.com/n0-computer/noq/commit/1350132407c7ded9c5e71d9b9b5eb13f2f095606))
- Split client and server hp state ([#214](https://github.com/n0-computer/noq/issues/214)) - ([fc35922](https://github.com/n0-computer/noq/commit/fc35922d6461b4c1e5c49ed3aef1379fd6d4f3a9))
- Use array based data structure for sent_packets instead of BTreeMap ([#202](https://github.com/n0-computer/noq/issues/202)) - ([bad9ca2](https://github.com/n0-computer/noq/commit/bad9ca2abd6d60e45befd97efcbe5b297bb106af))
- Create PathChallenge and PathResponse types ([#240](https://github.com/n0-computer/noq/issues/240)) - ([a688a24](https://github.com/n0-computer/noq/commit/a688a240720047ef9f31e590d8168b2a71b41ba4))
- Update to `draft-ietf-quic-multipath-18` ([#241](https://github.com/n0-computer/noq/issues/241)) - ([dd9430a](https://github.com/n0-computer/noq/commit/dd9430a61b37407b3ee1e5311fb1634c0661cb10))
- Move path logic for received path response to path ([#259](https://github.com/n0-computer/noq/issues/259)) - ([706cb2d](https://github.com/n0-computer/noq/commit/706cb2d23253c0d72c3ea8b28ee2518c4c908253))
- [**breaking**] Use NonZeroUsize for segments sizes - ([40fffcb](https://github.com/n0-computer/noq/commit/40fffcb74bb149fc640d1a96371bee73e3dddf74))
- [**breaking**] Use NonZeroUsize for segment sizes ([#251](https://github.com/n0-computer/noq/issues/251)) - ([d3cdbd4](https://github.com/n0-computer/noq/commit/d3cdbd425ba624fb393c93df759a0a86fff9d03a))
- Track 4-tuples instead of only the remote per path. - ([b978fdf](https://github.com/n0-computer/noq/commit/b978fdf4db7ce1d4d6c36b21efe9b38e61674cf3))
- Track 4-tuples instead of only the remote per path. ([#264](https://github.com/n0-computer/noq/issues/264)) - ([f44b87f](https://github.com/n0-computer/noq/commit/f44b87f934a0d01642306abd40648dcec7ade73c))

### 📚 Documentation

- Fixup readme - ([e8b9fb3](https://github.com/n0-computer/noq/commit/e8b9fb3f3442ccdb32e0b4dd4adfe4de761785d6))
- Cleanup readme and update licensing infos - ([afc6947](https://github.com/n0-computer/noq/commit/afc69473231d326b707a35fee2c6378ba3e093c8))
- Cleanup readme and update licensing infos ([#266](https://github.com/n0-computer/noq/issues/266)) - ([83b3c73](https://github.com/n0-computer/noq/commit/83b3c732398219e63b0d6a6f78d656f33df8bd83))

### ⚡ Performance

- *(iroh-quinn)* Add a mutable buffer so small writes are less expensive ([#249](https://github.com/n0-computer/noq/issues/249)) - ([695025c](https://github.com/n0-computer/noq/commit/695025c28e98ffeb99f14ace8d00b8c90b98256c))
- *(quinn-proto)* Avoid creating small Bytes when doing small writes via AsyncWrite::write ([#252](https://github.com/n0-computer/noq/issues/252)) - ([4cbbb68](https://github.com/n0-computer/noq/commit/4cbbb68f3c80a14b5300ca8c987691ab842ba9d3))
- Implement more efficient timeouts and MSRV to 1.83 - ([817a1b5](https://github.com/n0-computer/noq/commit/817a1b565db5d48a3d04f0f51dd108c1d75aad56))
- Add a feature to enable profiling with tokio-console - ([a9d4377](https://github.com/n0-computer/noq/commit/a9d4377db23110f4ed011c4c1f571d2e5005db84))
- Move main() above Cli - ([e4f0550](https://github.com/n0-computer/noq/commit/e4f0550b29cfaa2ade9752ae7aea8b801c7d3699))

### 🎨 Styling

- Use display for PathId in tracing fields ([#201](https://github.com/n0-computer/noq/issues/201)) - ([0805840](https://github.com/n0-computer/noq/commit/0805840549385a67cbc6e13346f72909ba2f3c69))

### 🧪 Testing

- *(quinn-proto)* Add proptests for assembler ([#243](https://github.com/n0-computer/noq/issues/243)) - ([5a231e3](https://github.com/n0-computer/noq/commit/5a231e3d91e540898ba462b757e47120b4597d62))
- Fix imports for fuzz targets ([#189](https://github.com/n0-computer/noq/issues/189)) - ([1b44365](https://github.com/n0-computer/noq/commit/1b44365f55c51594dfecec659140ab825b01fc7e))
- Assume low latency to avoid pacing issues - ([39c190f](https://github.com/n0-computer/noq/commit/39c190f526ff5f5ce6ce4e4f0cc99b0b0a82280d))
- Fixup pqc test - ([cdf316d](https://github.com/n0-computer/noq/commit/cdf316d44fe1a419715651751d64577809b15008))
- Add code to programmatically generate interaction patterns with quinn-proto ([#193](https://github.com/n0-computer/noq/issues/193)) - ([5e51c98](https://github.com/n0-computer/noq/commit/5e51c987830e0065444091792c944cf2cd4e7c40))

### ⚙️ Miscellaneous Tasks

- *(ci)* Cache more deps ([#253](https://github.com/n0-computer/noq/issues/253)) - ([3e1abca](https://github.com/n0-computer/noq/commit/3e1abcae86031c9bb3cf1430ec1440376779f864))
- *(deps)* Bump rcgen from 0.14.3 to 0.14.4 - ([da06312](https://github.com/n0-computer/noq/commit/da0631246457f8449c980523703a0524151687ee))
- *(deps)* Bump serde from 1.0.219 to 1.0.223 - ([f76410d](https://github.com/n0-computer/noq/commit/f76410dc70e6a04b20a543ad80244b49a574abe9))
- *(deps)* Bump async-io from 2.5.0 to 2.6.0 - ([aaf6354](https://github.com/n0-computer/noq/commit/aaf63545cbd309113a3f2b5612ced7beffe81dc7))
- *(deps)* Bump serde from 1.0.225 to 1.0.226 - ([93b6d01](https://github.com/n0-computer/noq/commit/93b6d01605147b9763ee1b1b381a6feb9fcd454e))
- *(deps)* Bump rcgen from 0.14.4 to 0.14.5 - ([2515f34](https://github.com/n0-computer/noq/commit/2515f343c0d7b8f41f4d0ef3ba57f7e8bff8562e))
- *(deps)* Bump windows-sys from 0.61.0 to 0.61.1 - ([3dfd4c5](https://github.com/n0-computer/noq/commit/3dfd4c560dff33f85f6e51f170b01df10f55c4d8))
- *(deps)* Bump serde from 1.0.226 to 1.0.228 - ([e340daa](https://github.com/n0-computer/noq/commit/e340daa36e252de9e43419bff5e5c65ea454dd15))
- *(deps)* Bump getrandom from 0.3.3 to 0.3.4 - ([1e44c69](https://github.com/n0-computer/noq/commit/1e44c69f866374c4b5bddc3a2d3d6ff328bb3634))
- *(deps)* Bump actions/setup-node from 4 to 6 - ([fc5e219](https://github.com/n0-computer/noq/commit/fc5e219fd9c9418c874f3db4e6d26d9cc32cea67))
- *(deps)* Bump rustls-platform-verifier from 0.6.1 to 0.6.2 - ([491b8b5](https://github.com/n0-computer/noq/commit/491b8b5deb14ff5c1e4ba709b7936855e63aeef5))
- *(deps)* Bump console-subscriber from 0.4.1 to 0.5.0 - ([425b8b4](https://github.com/n0-computer/noq/commit/425b8b45f60d50eacec387def6372a221e268dea))
- *(deps)* Bump hex-literal from 1.0.0 to 1.1.0 - ([7fc33f4](https://github.com/n0-computer/noq/commit/7fc33f4143f1084e9839f858177fc9cb7720a084))
- *(deps)* Bump bytes from 1.10.1 to 1.11.0 - ([ac22065](https://github.com/n0-computer/noq/commit/ac22065c2943e5578d191179d908ffcb0afa1b6e))
- *(deps)* Bump actions/checkout from 5 to 6 - ([d2cdd65](https://github.com/n0-computer/noq/commit/d2cdd65ca44208f4820ff612f72578b8461efe7e))
- Unused var - ([c232f40](https://github.com/n0-computer/noq/commit/c232f40b993d09ab48ee0d4833119925ea594e73))
- Only consider a path available if it has CIDs ([#173](https://github.com/n0-computer/noq/issues/173)) - ([6dcadf2](https://github.com/n0-computer/noq/commit/6dcadf2867a55d4b3b7425689f5ab4d20100a2c0))
- Fix wasm builds ([#186](https://github.com/n0-computer/noq/issues/186)) - ([7854c50](https://github.com/n0-computer/noq/commit/7854c50e0ea22cd3a6592315b13fc189a4358a23))
- Pin wasm-bindgen-cli to match lockfile - ([994ab35](https://github.com/n0-computer/noq/commit/994ab35b9f2d07f5ac940eb03c8ccc302d6f2198))
- Drop netbsd testing - ([eee334d](https://github.com/n0-computer/noq/commit/eee334dff6c2ada82e2125ae2e99b15ce6f4cbef))
- Hide implicit feature for optional smol dependency - ([9596c7a](https://github.com/n0-computer/noq/commit/9596c7a2611c6060d24af8c98bf0e2f0fe31a959))
- Fixup package names - ([6380cb3](https://github.com/n0-computer/noq/commit/6380cb33093cdcf6a20f23848196be01b49d6128))
- Fixup wasm builds - ([ec7c803](https://github.com/n0-computer/noq/commit/ec7c8035fcb6409a20a8fd03b987b2b52eea37df))
- Merge upstream ([#194](https://github.com/n0-computer/noq/issues/194)) - ([8954a64](https://github.com/n0-computer/noq/commit/8954a64a7f4aeb03c8c1327675f1d402432d54ae))
- Setup n0 based CI - ([a1cc539](https://github.com/n0-computer/noq/commit/a1cc539566fd5c9b6c6115322f42146b118a5c82))
- Fix more workflows - ([35f654b](https://github.com/n0-computer/noq/commit/35f654b0c04fa4369098f2b261160207b7198b0c))
- Run docs preview, as long as it's a run in the `n0-computer/quinn` repo ([#256](https://github.com/n0-computer/noq/issues/256)) - ([52364f3](https://github.com/n0-computer/noq/commit/52364f3f3a4210dc227d275ccc78c36ab44001fb))
- Update branch name to main - ([6ed0e45](https://github.com/n0-computer/noq/commit/6ed0e458f94feb1f86912bc6ee0266720c258e12))
- Update branch name to main ([#265](https://github.com/n0-computer/noq/issues/265)) - ([8e056b5](https://github.com/n0-computer/noq/commit/8e056b5bfe99b0340316bd05c0db86cba82d9387))
- Follow discord notification conventions ([#297](https://github.com/n0-computer/noq/issues/297)) - ([1240596](https://github.com/n0-computer/noq/commit/124059612a8149fe973a3f8629ec84c4773af1de))
- Release v0.15.0 - ([d528745](https://github.com/n0-computer/noq/commit/d5287455b6c7565c831380192d94618410d4cb02))

### Fix

- Use the local max path ID instead of `Connection::max_path_id` in receive paths - ([b0e69b5](https://github.com/n0-computer/noq/commit/b0e69b534bb600ea3c0e2a27662aab950b2dc172))

### Deps

- Switch to criterion for benching ([#262](https://github.com/n0-computer/noq/issues/262)) - ([65c66d5](https://github.com/n0-computer/noq/commit/65c66d5929a1ab39b780dcdadff03d54c26c6fa7))

### Quinn

- Drop async-std support - ([04c1aaf](https://github.com/n0-computer/noq/commit/04c1aaf1e6666c7f03bbaf3e6248c27b5f17c814))
- Track callers in runtime spawn abstraction - ([ecae4ad](https://github.com/n0-computer/noq/commit/ecae4ad100481d1efc248ec214b8105502106ec2))
- Drop async-std support - ([d845f40](https://github.com/n0-computer/noq/commit/d845f4009de02ab710b6b23c3b7e4d2fc63b5742))
- Move runtime module to be closer to its child modules - ([42bc17f](https://github.com/n0-computer/noq/commit/42bc17f812bf8797de9188f54f0070674e85f7ee))
- Rename async_io module to smol - ([4c17321](https://github.com/n0-computer/noq/commit/4c17321999561b33d7ba4f9bce75a0d368eb0435))
- Simplify cfg guards for runtime-smol - ([1ffd9cd](https://github.com/n0-computer/noq/commit/1ffd9cd83fef6970c34d26320bfd790f8b34b062))
- Statically hide default_runtime() if unavailable - ([f90254a](https://github.com/n0-computer/noq/commit/f90254a4107d3cb600b83ca2e536ecba5023d051))

## [quinn-udp-0.5.14](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.13..quinn-udp-0.5.14) - 2025-08-27

### ⛰️  Features

- Add `Connection::set_send_window()` - ([8fb2a07](https://github.com/n0-computer/noq/commit/8fb2a07e199dc7681da2f0bbe27dc5d3a099fd2b))

### 🐛 Bug Fixes

- Errors from merging - ([c5b5567](https://github.com/n0-computer/noq/commit/c5b556736cf8e5297b3d52eb58144318ca490dd6))

### 📚 Documentation

- *(quinn)* Revise SendStream::write docs - ([ab953ab](https://github.com/n0-computer/noq/commit/ab953abeb3f02dd4ea729c15236b06ae6e706a59))

### ⚡ Performance

- Allow setting initial round trip time - ([abd1be0](https://github.com/n0-computer/noq/commit/abd1be051b64ecb7f882d2967141c6e2f7f50401))
- Allow configuring ack frequency - ([1678ada](https://github.com/n0-computer/noq/commit/1678ada26d442eaa48e341cff51a3d47f5ae3f90))
- Allow selecting congestion algorithm - ([a8eba3a](https://github.com/n0-computer/noq/commit/a8eba3ada638b6c9c87c9f5e249265b6fb6fcf90))
- Leave async tasks early - ([62bc881](https://github.com/n0-computer/noq/commit/62bc881b9a7b8f6e95950304672af2d497a9ab32))
- Factorize common options - ([c5da146](https://github.com/n0-computer/noq/commit/c5da14658a5ed26cca9664c075d3b13bcb6221cd))
- Factorize socket binding - ([0c8b65f](https://github.com/n0-computer/noq/commit/0c8b65fd42af7d2dac539850e9f91403e137509f))
- Used buffered writer for qlog stream - ([7825e9d](https://github.com/n0-computer/noq/commit/7825e9d39e277f58659fc21c69e8055a08ad92ed))
- Factorize init of tracing - ([7d59865](https://github.com/n0-computer/noq/commit/7d59865ffe6b0862a5cdb9e1418e941730d166c3))
- Support SI units for the various size parameters - ([325b4e0](https://github.com/n0-computer/noq/commit/325b4e0e42575d0a3486d1d484c13281278aee21))
- Allow setting send / receive connection or stream windows - ([8ba63e4](https://github.com/n0-computer/noq/commit/8ba63e4d713c0e895b6f5a86aba402c8db7b015a))
- Add max udp payload size option - ([c7bdfa2](https://github.com/n0-computer/noq/commit/c7bdfa2b8f64693a718aa170f6405f5574f57147))
- Move client and server on one binary - ([f5632ce](https://github.com/n0-computer/noq/commit/f5632ce2ee7d8f29197b3547edb59558513a3fab))
- Rename perf binary to quinn-perf to prevent name clash with linux perf - ([3194714](https://github.com/n0-computer/noq/commit/3194714f6fa1c78108b2a6353bd3e2af95ce1152))

### ⚙️ Miscellaneous Tasks

- *(Cargo.toml)* Support both socket2 v0.5 and v0.6 - ([253b388](https://github.com/n0-computer/noq/commit/253b3885f6857ada5d717bf176251d6219410a0a))
- *(deps)* Bump rcgen from 0.14.0 to 0.14.1 - ([a134be0](https://github.com/n0-computer/noq/commit/a134be0de61ab3f76fe00d991942a350124314bf))
- *(deps)* Bump rcgen from 0.14.1 to 0.14.2 - ([28a4532](https://github.com/n0-computer/noq/commit/28a4532c92d0f8add909a4eacd1a77a0b8b1b84c))
- *(deps)* Bump serde_json from 1.0.140 to 1.0.141 - ([f5f3490](https://github.com/n0-computer/noq/commit/f5f34905f8d67228b706650c657ab13e07a12abe))
- *(deps)* Bump async-io from 2.4.1 to 2.5.0 - ([3a1300a](https://github.com/n0-computer/noq/commit/3a1300a61ac8280889e4d8e631a64b9ec6729c57))
- *(deps)* Bump rcgen from 0.14.2 to 0.14.3 - ([1f8d2c2](https://github.com/n0-computer/noq/commit/1f8d2c26b7bb9d152c7ca6807dc1a51a76c13083))
- *(deps)* Bump criterion from 0.6.0 to 0.7.0 - ([62f004b](https://github.com/n0-computer/noq/commit/62f004b8db3913ff85872899a81701d5d6db481e))
- *(deps)* Bump actions/checkout from 4 to 5 - ([b6e4257](https://github.com/n0-computer/noq/commit/b6e42576f0bbb74b1578233c47e52eff77bd21e9))
- *(deps)* Bump rustls-platform-verifier from 0.6.0 to 0.6.1 - ([20af7c7](https://github.com/n0-computer/noq/commit/20af7c7b438dc733beebd353bf14bccedf8f22da))
- *(deps)* Bump serde_json from 1.0.141 to 1.0.142 - ([aa41c90](https://github.com/n0-computer/noq/commit/aa41c901b2ddd09a2ed02d947d2029452908d8be))
- *(deps)* Bump windows-sys from 0.59.0 to 0.60.2 - ([bb46d09](https://github.com/n0-computer/noq/commit/bb46d09f00c14ddedca9c88dc961225ac7dd4a4e))
- *(deps)* Bump url from 2.5.4 to 2.5.7 - ([d50768f](https://github.com/n0-computer/noq/commit/d50768f085141b788dac4fd9092962695520ae0a))
- *(deps)* Bump actions/setup-java from 4 to 5 - ([476f769](https://github.com/n0-computer/noq/commit/476f7690e7cc679bb694f40fef59c74c5b88dbd1))
- *(deps)* Bump serde_json from 1.0.142 to 1.0.143 - ([1b4d576](https://github.com/n0-computer/noq/commit/1b4d576a62360685929ad4a0ec9c63c108ce27c1))
- Remove individual repo project tracking ([#34](https://github.com/n0-computer/noq/issues/34)) - ([d579a38](https://github.com/n0-computer/noq/commit/d579a38fab6b8a00f3925e966b02787486047676))
- Add test cases for `set_send_window` - ([f6baf26](https://github.com/n0-computer/noq/commit/f6baf26aae3fa154120a019fc63b4d9c3c0ea3de))

### Bench

- Refactor parse_byte_size - ([ce94052](https://github.com/n0-computer/noq/commit/ce94052e3bc86d83cf7ed936b44ee1b1325c3418))
- Fix clap help - ([7aa7c8d](https://github.com/n0-computer/noq/commit/7aa7c8d79d0629bc3b71e6ffe0fc11508052705b))

### Cov

- Fix qlog feature name - ([f31b16d](https://github.com/n0-computer/noq/commit/f31b16d43a5a82c7df5490d3c44b7c6d9cf33ca9))

### Proto

- Upgrade to rustls-platform-verifier 0.6 - ([e8fa804](https://github.com/n0-computer/noq/commit/e8fa80432ff0d615deb1942fb0e9c20f9dee98e3))
- Add option to pad application data UDP datagrams to MTU - ([6fb6b42](https://github.com/n0-computer/noq/commit/6fb6b424d78d46d22c10cb3b788478163b0bfffd))
- Fix NewConnectionId size bound - ([446cd72](https://github.com/n0-computer/noq/commit/446cd72e38b079ccd0c867138044e073e87ca03c))
- Inline StepRng implementation from rand - ([926e78d](https://github.com/n0-computer/noq/commit/926e78dfb68e41c7e378d37fbe02a2d615308247))

### Wip

- Allow server migrations - ([bc86957](https://github.com/n0-computer/noq/commit/bc86957aa4ccb72fad70e75a6ce9fc8198f09afc))

## [quinn-udp-0.5.13](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.12..quinn-udp-0.5.13) - 2025-06-18

### ⛰️  Features

- *(quinn-proto)* Validate paths_blocked and path_cids_blocked frames - ([1ccc1dc](https://github.com/n0-computer/noq/commit/1ccc1dc4f859a8aface21b1adb6439b0d658f6b0))
- *(quinn-proto)* Handle receiving of path availability frames ([#77](https://github.com/n0-computer/noq/issues/77)) - ([63d254b](https://github.com/n0-computer/noq/commit/63d254b996108cb50b11e30665ac9ce19b6cb747))
- Make the future returned from SendStream::stopped 'static - ([f1fe183](https://github.com/n0-computer/noq/commit/f1fe1832a7badcefd828f130753b6dec181020a2))
- Initial setup for high level open path api ([#81](https://github.com/n0-computer/noq/issues/81)) - ([e9e94af](https://github.com/n0-computer/noq/commit/e9e94af1a96c1075666c0cb5a11566777fea309d))
- Merge iroh-0.11.x ([#109](https://github.com/n0-computer/noq/issues/109)) - ([e052000](https://github.com/n0-computer/noq/commit/e05200036bfb3c68bf51b3122ebe6471c4002fff))
- Add `upgrade` function for weak connection handle - ([0d929df](https://github.com/n0-computer/noq/commit/0d929df5f69ddc660c8ce81e9c348af7972862db))
- Add `upgrade` function for weak connection handle ([#110](https://github.com/n0-computer/noq/issues/110)) - ([70e2887](https://github.com/n0-computer/noq/commit/70e28875923db76f8dfbf4f058e682d56e6daea1))

### 🐛 Bug Fixes

- *(ci)* Delete repo level project tracking ([#72](https://github.com/n0-computer/noq/issues/72)) - ([12218f9](https://github.com/n0-computer/noq/commit/12218f93dbfe52dbde4b2b2526f285a2c4acc121))
- *(quinn-proto)* Improve per path logic in poll_transmit - ([e8a83c4](https://github.com/n0-computer/noq/commit/e8a83c4e6c79ba019b74fcbbf7e77b93ef4f1033))
- *(quinn-proto)* Correctly associate ACKs to paths ([#79](https://github.com/n0-computer/noq/issues/79)) - ([339b6cf](https://github.com/n0-computer/noq/commit/339b6cf9045d4e2198ff8c9eacfb99e2246e8362))

### 🚜 Refactor

- *(quinn)* Remove some usage of execute_poll - ([4f8a0f1](https://github.com/n0-computer/noq/commit/4f8a0f13cf7931ef9be573af5089c7a4a49387ae))
- *(quinn)* Configure out `async_io::UdpSocket` when unused - ([e8dc5a2](https://github.com/n0-computer/noq/commit/e8dc5a2eda57163bfbaba52ba57bf5b7a0027e22))
- Improve path ack handling - ([4fad7bb](https://github.com/n0-computer/noq/commit/4fad7bb7cffdbdace8848499e780322aaf6c3d6c))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump getrandom from 0.3.2 to 0.3.3 - ([81282af](https://github.com/n0-computer/noq/commit/81282af8d5d27859f1a3324cf3a1884434f7965a))
- *(deps)* Bump rustls-platform-verifier from 0.5.1 to 0.5.3 - ([176e84c](https://github.com/n0-computer/noq/commit/176e84c66698f112dc8f322e47d5fd7a6b23d0b4))
- *(deps)* Bump socket2 from 0.5.9 to 0.5.10 - ([9fd189c](https://github.com/n0-computer/noq/commit/9fd189c7d5bf08d543b03a29bf0913d6909ec569))
- *(deps)* Bump async-io from 2.4.0 to 2.4.1 - ([f61a0f6](https://github.com/n0-computer/noq/commit/f61a0f6637803007aaf591b0ec1384d1610b6c66))
- *(deps)* Bump criterion from 0.5.1 to 0.6.0 - ([0699545](https://github.com/n0-computer/noq/commit/06995454f44171d4164753b95e0bce900089a9a7))
- *(udp)* Increase patch version to v0.5.13 - ([113fa61](https://github.com/n0-computer/noq/commit/113fa61de3fb4ff1c3622e53f530bd8d84d0a3bf))
- Remove explicit write future structs - ([bce3284](https://github.com/n0-computer/noq/commit/bce32845dcb0a466a4e0e1b01c2a9cdf0bc5bf54))
- Suppress large AcceptError clippy warning - ([c8ca79c](https://github.com/n0-computer/noq/commit/c8ca79c9c318e6a27e573e3b301193eff1c5463a))
- Fixup - ([1bc4f51](https://github.com/n0-computer/noq/commit/1bc4f5176a2e2e8823390d5419f08d95042214d0))

### Proto

- Make BytesSource private - ([9f008ad](https://github.com/n0-computer/noq/commit/9f008ade668c1f0112affd55f4ce7d325f697c27))

### Udp

- Disable GSO for old Linux - ([81f9cd9](https://github.com/n0-computer/noq/commit/81f9cd99579f6e33ca03c4ec1cbb4fba5c3e5273))

## [quinn-proto-0.11.12](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.12..quinn-proto-0.11.12) - 2025-05-09

### 🧪 Testing

- *(proto)* Use default TokenMemoryCache - ([1126591](https://github.com/n0-computer/noq/commit/11265915ae8c58dde53dca9af57bc0946ef23bb9))
- *(proto)* Use default BloomTokenLog - ([7ce43e8](https://github.com/n0-computer/noq/commit/7ce43e8e7b22c61fee3430d1c1a1bf447e046e02))

### ⚙️ Miscellaneous Tasks

- Change powerset check - ([f642fa8](https://github.com/n0-computer/noq/commit/f642fa870edb4339e3135ef438eed1c43d03073a))

### Proto

- Bump version to 0.11.12 - ([3482fcc](https://github.com/n0-computer/noq/commit/3482fcc759675ebb16348826ee88e77d764a4900))

### Quinn

- Fix feature combination error / warnings - ([14b905a](https://github.com/n0-computer/noq/commit/14b905ae568ab050caa63954673a2d99cf8e0497))

## [quinn-udp-0.5.12](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.11..quinn-udp-0.5.12) - 2025-04-30

### 🐛 Bug Fixes

- *(udp)* Zero control message array on fast-apple-datapath - ([76b8916](https://github.com/n0-computer/noq/commit/76b89160fa74a23717e8bc97507397a18dadcc90))

### ⚙️ Miscellaneous Tasks

- *(udp)* Increase crate patch version to v0.5.12 - ([458295c](https://github.com/n0-computer/noq/commit/458295c30519f56ec160cc9c6264df72e2601e45))
- Sanitise `segment_size` - ([6b901a3](https://github.com/n0-computer/noq/commit/6b901a3c278f58497d6d53c64ef1cc53497c625b))

## [quinn-proto-0.11.11](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.11..quinn-proto-0.11.11) - 2025-04-22

### 🐛 Bug Fixes

- *(quinn-udp)* Move cmsg-len check to Iterator - ([19a625d](https://github.com/n0-computer/noq/commit/19a625de606ea8e83bbf8e5c9265f21ebef193da))
- Respect max_datagrams when tail-loss probes happen and initial mtu is large enough to batch - ([cc7608a](https://github.com/n0-computer/noq/commit/cc7608a6be9153267ded63cd669a7dff54732226))

### 🚜 Refactor

- *(quinn-udp)* Favor early-return for `send` impls - ([56e19b8](https://github.com/n0-computer/noq/commit/56e19b841f02ebc8c3982dcee47839563a228740))
- *(quinn-udp)* Favor early-return for `recv` impls - ([3391e7a](https://github.com/n0-computer/noq/commit/3391e7a4a6e1d30b68037247480a5a98c8defe2e))
- *(quinn-udp)* Avoid blocks in `match` arms - ([075c7ef](https://github.com/n0-computer/noq/commit/075c7ef235f2acbf7cf4ba2b203b1c4448e6a0f2))
- *(quinn-udp)* Remove redundant match-arms - ([3e81eb0](https://github.com/n0-computer/noq/commit/3e81eb0dfb2c49b18170533339f0d673e277a51b))
- *(quinn-udp)* Use `match` blocks in `recv` - ([c7687f7](https://github.com/n0-computer/noq/commit/c7687f7e0c5340168a29c348a4b794b66beee814))
- Move max_datagrams limit at poll_transmit from quinn-proto to quinn - ([f8165c3](https://github.com/n0-computer/noq/commit/f8165c339483a09204514377c430579ceb6509e5))

### 📚 Documentation

- *(quinn)* Copy edit poll_read(_buf?) docs - ([37beebf](https://github.com/n0-computer/noq/commit/37beebfa08e7e3cf66507ecbe611d540c5812cc1))
- Separate example code from document - ([41f7d2e](https://github.com/n0-computer/noq/commit/41f7d2ea8f645adf630ca5712259fa34770c331e))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump socket2 from 0.5.8 to 0.5.9 - ([c94fa9b](https://github.com/n0-computer/noq/commit/c94fa9bacbb71bfd737245539e678e9be9be7d66))
- *(deps)* Bump rand from 0.9.0 to 0.9.1 - ([b406b98](https://github.com/n0-computer/noq/commit/b406b98e45607ce2f8e9e4c2d08540419bfea6eb))
- Rely on implicit targets - ([ab0596a](https://github.com/n0-computer/noq/commit/ab0596a89ba8e137add9d5e9a0ab54cda17dc58b))
- Specify dependency versions - ([2f60681](https://github.com/n0-computer/noq/commit/2f60681abe8d626b2a15a42042fac479fd391168))
- Run macOS tests conditionally on runner OS - ([107dd92](https://github.com/n0-computer/noq/commit/107dd923759419d5eaacde5323338b0b77310f20))
- Run `quinn-udp` fast-data-path tests - ([3f94660](https://github.com/n0-computer/noq/commit/3f9466020cff6f846550fdfc9c1d923fc53c29ca))

### WIP

- Send on multiple paths - ([d056825](https://github.com/n0-computer/noq/commit/d056825cd10e7f4303481a4daa88b584b29e85f0))

### Book

- Fix example code - ([eec45e6](https://github.com/n0-computer/noq/commit/eec45e6b7629f76605966c7018eb37991b829976))
- Fix code references - ([bbf9510](https://github.com/n0-computer/noq/commit/bbf95101cd5b7e54b1930d0d64951aa566f2283c))
- Clean up formatting - ([3d019b3](https://github.com/n0-computer/noq/commit/3d019b3fd5be749178e28a0bf429af430ea7cffd))
- Suppress warnings in code samples - ([3610629](https://github.com/n0-computer/noq/commit/3610629113fcca464dc22199f9e1e5c8e7d50f92))
- Merge certificate code files - ([2447c2e](https://github.com/n0-computer/noq/commit/2447c2e65114eb6589db8e96183551985f99721b))
- Rename certificate-insecure to certificate - ([62fc039](https://github.com/n0-computer/noq/commit/62fc0397fb14db94d1ec27a0ca63476469a5f67e))
- Import more types - ([48e0bb3](https://github.com/n0-computer/noq/commit/48e0bb3317b13364aa94319431f9dc5d34b478a4))
- Order certificate code in top-down order - ([d948de6](https://github.com/n0-computer/noq/commit/d948de66b5ff43e1545f46bb38bfaf8e78189224))
- Simplify connection setup constants - ([a196f7c](https://github.com/n0-computer/noq/commit/a196f7c48049c7e26ed51449f3ab3f0746e88ce7))
- Order set-up-connection code in top-down order - ([6b6d115](https://github.com/n0-computer/noq/commit/6b6d115bdace983ecd0cb8bdcc24f7e19c280e47))
- Order data-transfer code in top-down order - ([a788429](https://github.com/n0-computer/noq/commit/a788429e919d8e3a1563641d44d5c032be74221c))
- Remove unused dependency - ([e960c33](https://github.com/n0-computer/noq/commit/e960c33729660013d5d1436a37d19994f0b7034d))
- Remove obsolete rustls features - ([f63d962](https://github.com/n0-computer/noq/commit/f63d962d0829799f8775da70d0659a43c457159f))

### Quinn-udp

- Handle EMSGSIZE in a common place - ([8f1a529](https://github.com/n0-computer/noq/commit/8f1a529837c7c99741d4097446a85e4482bf65b3))

## [quinn-udp-0.5.11](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.10..quinn-udp-0.5.11) - 2025-03-25

### ⛰️  Features

- Allow changing the UDP send/receive buffer sizes - ([83b48b5](https://github.com/n0-computer/noq/commit/83b48b5b87faa2033fd7a2c824aa108baf6d3569))

### 🐛 Bug Fixes

- Do not produce tail-loss probes larger than segment size - ([434c358](https://github.com/n0-computer/noq/commit/434c35861e68aac1da568bcd0b1523603f73f255))

## [quinn-proto-0.11.10](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.10..quinn-proto-0.11.10) - 2025-03-08

### Proto

- Fix compatibility with older quinn - ([a7821ff](https://github.com/n0-computer/noq/commit/a7821ff3da0884f42bad3a1b21ab96ff998c4f68))

## [quinn-udp-0.5.10](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.9..quinn-udp-0.5.10) - 2025-02-14

### ⛰️  Features

- *(quinn,quinn-udp)* Disable `socket2` and `std::net::UdpSocket` dependencies in wasm/browser targets - ([a5e9504](https://github.com/n0-computer/noq/commit/a5e950495220ee3c761371fb540764e2c4743ab8))

### 🐛 Bug Fixes

- *(.github/codecov)* Ignore aws-lc-rs-fips for codecov - ([7d87dc9](https://github.com/n0-computer/noq/commit/7d87dc9f6ab5d7834ad1d21c3c2ef87eeac921c7))
- *(quinn)* `impl tokio::io::AsyncWrite for SendStream` - ([13decb4](https://github.com/n0-computer/noq/commit/13decb40b3a07af8bb9c46fb3beb6d08f81f86e5))
- *(udp/fast-apple)* Ignore empty cmsghdr - ([f582bc8](https://github.com/n0-computer/noq/commit/f582bc8036522d475c22c201e0b3b5533dbccf6c))
- *(udp/windows)* Set socket option IPV6_RECVECN - ([c32e2e2](https://github.com/n0-computer/noq/commit/c32e2e20896e6e1c78222cfcc703c3d36722bfb2))
- *(udp/windows)* Set socket option IP_RECVECN - ([fbc795e](https://github.com/n0-computer/noq/commit/fbc795e3cea722996232f2c853772390e05d51fe))

### 🧪 Testing

- *(proto)* Refactor IncomingConnectionBehavior - ([5a572e0](https://github.com/n0-computer/noq/commit/5a572e067d38b368a1955ae92921d4901aab8b4e))
- *(proto)* Enable NEW_TOKEN usage in tests - ([ee29715](https://github.com/n0-computer/noq/commit/ee297152155ee3bb6a480fff7618e56061de9908))
- *(proto)* Create tests::token module - ([d2acbc3](https://github.com/n0-computer/noq/commit/d2acbc3e94037d6d079abb8bc998bc147fab03bf))
- *(proto)* Add tests for NEW_TOKEN frames - ([bb54bc4](https://github.com/n0-computer/noq/commit/bb54bc4a51594c86d757fb710b23e0a8a6f1d7fb))
- *(proto)* Fix wasm CI - ([69c00eb](https://github.com/n0-computer/noq/commit/69c00ebfdc589f574dd3a515db700948086f3a83))
- Avoid ICE in beta - ([6bfd248](https://github.com/n0-computer/noq/commit/6bfd24861e65649a7b00a9a8345273fe1d853a90))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update rustls-platform-verifier requirement from 0.4 to 0.5 - ([7cc1db2](https://github.com/n0-computer/noq/commit/7cc1db2cbc52f518c5457f4550b17d17a10efb88))
- *(udp)* Increase crate patch version to v0.5.10 - ([f4bd4c2](https://github.com/n0-computer/noq/commit/f4bd4c21f4dec001d044ba4cd279b91627124b01))
- Factor out IncomingToken - ([89f3f45](https://github.com/n0-computer/noq/commit/89f3f458de2a39e9eb4ff040ee15d22250192d3d))
- Remove panic hazards from RetryToken decode - ([bde7592](https://github.com/n0-computer/noq/commit/bde7592ea51ef0c7be39b6c2865bded9e4bada64))
- Run quinn-udp tests with fast-apple-datapath - ([3c3d460](https://github.com/n0-computer/noq/commit/3c3d46037884b0bf2b7d64653f88681381489eea))
- Powerset --clean-per-run - ([d5e63d8](https://github.com/n0-computer/noq/commit/d5e63d8c2869af9f5e8af7492b42696cab55848f))
- Change how tokens are encrypted - ([b237cd7](https://github.com/n0-computer/noq/commit/b237cd766e808e17f893ddf573b6a08a655d98c2))

### WIP

- Issue per-path CIDs and keep track of them - ([882fdf2](https://github.com/n0-computer/noq/commit/882fdf2370cd88d0f44ab080e20bbf5d44dade45))
- Issue per-path CIDs and keep track of them ([#47](https://github.com/n0-computer/noq/issues/47)) - ([1e3b7e8](https://github.com/n0-computer/noq/commit/1e3b7e8e59515505d12f0a888861b6bd6a99709b))
- Send CIDs for new paths correctly, if requested - ([ecd92e8](https://github.com/n0-computer/noq/commit/ecd92e8c613b4ee3354f6294909e6a6ca9662a2e))

### Proto

- Factor out IncomingToken::from_header - ([afc7d7f](https://github.com/n0-computer/noq/commit/afc7d7f8ae3ef690e7da4db7beadd6c1b07eae03))
- Replace hidden field with From impl - ([43b74b6](https://github.com/n0-computer/noq/commit/43b74b658b7038c9190c06e6969d16b82f9fc64b))
- Inline trivial constructor - ([8a488f2](https://github.com/n0-computer/noq/commit/8a488f2d7eb565d33daa5416ba57ce7b94f1401f))
- Inline IncomingToken::from_retry() - ([268cbd9](https://github.com/n0-computer/noq/commit/268cbd9116b078b61736053342cd41b7d5cafe95))
- Re-order items in token module - ([670c517](https://github.com/n0-computer/noq/commit/670c517f429ce3ca0893fa872334dc021d178c39))
- Un-hide EcnCodepoint variants - ([37b9340](https://github.com/n0-computer/noq/commit/37b93406cde5f6197c0aeaad5c4dfb36f5492b82))
- Remove superfluous `#[doc(hidden)]` fuzzing - ([16f83d1](https://github.com/n0-computer/noq/commit/16f83d1c8fa449f49ef63187bdb8415580a637ff))
- Pass SocketAddr by value - ([2071704](https://github.com/n0-computer/noq/commit/20717041bc308f88e99e35667737d6b51911a8b3))
- Utilize let-else in Endpoint::handle - ([c1aa2a8](https://github.com/n0-computer/noq/commit/c1aa2a8be8d85eead94ec7b7a69556edb106d6b9))
- Refactor Endpoint::handle - ([b350bb1](https://github.com/n0-computer/noq/commit/b350bb1b156e9beb3dd2202eb276dbc826f06413))
- Use pre-existing variable in handle - ([b1e7709](https://github.com/n0-computer/noq/commit/b1e77091eae6139d08ff546c5123f90b1a6692c6))
- Factor out return in handle - ([f99ca19](https://github.com/n0-computer/noq/commit/f99ca19bfe24713799decd60facf140ca9c42b22))
- Pass ConnectionId by value internally - ([7caa30b](https://github.com/n0-computer/noq/commit/7caa30bd6153264d698592c5d9df5d5ae029598d))
- Rename RetryToken::from_bytes -> decode - ([b0e39a9](https://github.com/n0-computer/noq/commit/b0e39a97fc18743fdec343e481a700355fff101e))
- Factor out encode_ip - ([8fd8e1a](https://github.com/n0-computer/noq/commit/8fd8e1a7c89ab4a95675880063bed603530fefcf))
- Factor out encode_unix_secs - ([371f180](https://github.com/n0-computer/noq/commit/371f18032d2d3ec1f59169d6e44e95ba5989011a))
- Simplify encode_unix_secs - ([5b45184](https://github.com/n0-computer/noq/commit/5b4518446b039591ef8b151d50b44a5b0761da8b))
- Remove Cursor usage from token.rs - ([5c381aa](https://github.com/n0-computer/noq/commit/5c381aab52cc96fd24bdcdfc8efa85ae1157e2e3))
- Rearrange lines of RetryToken::encode - ([e6380df](https://github.com/n0-computer/noq/commit/e6380df4867df3d4ea3b6fb20c5aa539c63c0b6c))
- Make address a field of RetryToken - ([6925099](https://github.com/n0-computer/noq/commit/692509900b0302528b49cdec8caa00534e99b181))
- Remove ValidationError - ([fe67e7c](https://github.com/n0-computer/noq/commit/fe67e7cd6499988d577d4e2adc826ab82e9f7a68))
- Reject RetryToken with extra bytes - ([bfbeecd](https://github.com/n0-computer/noq/commit/bfbeecdc1c23c4ba4e7697b67e4888a80b533fdb))
- Move more logic into handle_first_packet - ([408b7b0](https://github.com/n0-computer/noq/commit/408b7b0d44d8316851de649d4e6cff301f895fa1))
- Reduce whitespace in Endpoint.handle - ([7f11d3c](https://github.com/n0-computer/noq/commit/7f11d3cc716ce53e204bb72068d04e9e65fdb7e6))
- Almost always construct event in handle - ([ff2079b](https://github.com/n0-computer/noq/commit/ff2079b6a3616af2b856d5e8a388bbc632500ae8))
- Use event as param to handle_first_packet - ([1e7358c](https://github.com/n0-computer/noq/commit/1e7358c57dc96960b00d743660dee48a501b0a03))
- Remove most return statements from handle - ([3e3db6f](https://github.com/n0-computer/noq/commit/3e3db6f8665c1780f9ff7e22cc9f89f92aab5359))
- Use match for grease with reserved version - ([ffbd15f](https://github.com/n0-computer/noq/commit/ffbd15f087262893e8d319534b99b684c0091f50))
- Remove redundant cursors - ([23b18f2](https://github.com/n0-computer/noq/commit/23b18f2882ec0f55b491848c572a15344d599ec2))
- Replace calls to Duration::new - ([f5b1ec7](https://github.com/n0-computer/noq/commit/f5b1ec7dd96c9b56ef98f2a7a91acaf5e341d718))
- Factor out NewToken frame struct - ([273f7c2](https://github.com/n0-computer/noq/commit/273f7c23865df886f62f06ae8e22e168860d81e0))
- Rename RetryToken -> Token - ([df22e27](https://github.com/n0-computer/noq/commit/df22e2772ea0ba9408b49d01eed361647622590b))
- Split out RetryTokenPayload - ([22c1270](https://github.com/n0-computer/noq/commit/22c12708f0e9bb9087208e2c8d68d53fed512dc6))
- Convert TokenPayload into enum - ([78bfa5b](https://github.com/n0-computer/noq/commit/78bfa5b509465743954960d3aa549b61c148ce6b))

### Quinn

- Make SendStream::poll_stopped private - ([506e744](https://github.com/n0-computer/noq/commit/506e74417ac27e615cddda731d6b3218f383540d))

### Token

- Move RetryToken::validate() to IncomingToken::from_retry() - ([020c38b](https://github.com/n0-computer/noq/commit/020c38b1b7eb4bf343ab428cdc91ae1c56566ac2))

### Udp

- Un-hide EcnCodepoint variants - ([f51c93f](https://github.com/n0-computer/noq/commit/f51c93f2c21a0a1a6039a746f829d931909944c3))
- Tweak EcnCodepoint::from_bits - ([3395458](https://github.com/n0-computer/noq/commit/33954582da3193a8469bbb06fac04674c529555e))

## [quinn-udp-0.5.9](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.8..quinn-udp-0.5.9) - 2024-12-17

### ⛰️  Features

- *(quinn-proto)* Unhide `quinn_proto::coding` - ([7647bd0](https://github.com/n0-computer/noq/commit/7647bd01dd137d46a796fd6b766e49deda23c9d7))

### 🐛 Bug Fixes

- *(udp)* Retry send on first EINVAL - ([e953059](https://github.com/n0-computer/noq/commit/e9530599948820bd6bf3128e09319cd5eefc60ab))
- *(udp)* Make GRO (i.e. URO) optional, off by default - ([6ee883a](https://github.com/n0-computer/noq/commit/6ee883a20cb02968ae627e2ca9396f570d815e86))

### 🚜 Refactor

- *(udp)* Remove unnecessary `return` - ([cb0b59d](https://github.com/n0-computer/noq/commit/cb0b59d09c37836d44a9f591899490c0545360e1))

### 📚 Documentation

- *(quinn)* Tweak Connecting docs - ([04b9611](https://github.com/n0-computer/noq/commit/04b9611aff7d0da898ce2b42a5ddf3db19c9a5e1))

### 🧪 Testing

- *(proto)* Gate PLPMTUD test - ([caf8389](https://github.com/n0-computer/noq/commit/caf838947c59ec90ccb7a555cc9eb3ef39025232))
- *(quinn)* Ignore stress tests by default - ([6716b5a](https://github.com/n0-computer/noq/commit/6716b5a7b8c5c2e64522d56682ac12aae824c4cf))

### ⚙️ Miscellaneous Tasks

- *(udp)* Increase crate patch version to v0.5.9 - ([b720c6a](https://github.com/n0-computer/noq/commit/b720c6a1d3abe039aa8b826d054ef241cb05df7e))
- Re-ignore stress tests in solaris - ([db4c0e4](https://github.com/n0-computer/noq/commit/db4c0e40da25482a54c5fd0dbb7c75eda1ac28e0))
- Make Connection internally use SideState - ([e706cd8](https://github.com/n0-computer/noq/commit/e706cd8ac063dfa9d9843d54d69c5a9a7067d1e3))
- Make Connection externally use SideArgs - ([c5f81be](https://github.com/n0-computer/noq/commit/c5f81bec9bac9dcb894720689d4d938eea3fe569))
- Run on Android API Level 25 - ([a83c6e4](https://github.com/n0-computer/noq/commit/a83c6e463b0dd091582e2cbd76f970c690e12294))

### Proto

- Refactor TokenDecodeError - ([51e974e](https://github.com/n0-computer/noq/commit/51e974e4d9c7a1156c55e8510d07980832a7ef53))

### Quinn

- Remove obsolete must_use for futures - ([8ab077d](https://github.com/n0-computer/noq/commit/8ab077dbcecf2919bd3652a806176ec1d05f16b2))

## [quinn-udp-0.5.8](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.9..quinn-udp-0.5.8) - 2024-12-06

### #2057

- Use randomly generated GREASE transport parameter. - ([2edf192](https://github.com/n0-computer/noq/commit/2edf192511873a52093dd57b9e70eb4b27c442cd))
- Extract known transport parameter IDs into enum. - ([af4f29b](https://github.com/n0-computer/noq/commit/af4f29b8455590652c559fce1e923363ce8fae5a))
- Write transport parameters in random order. - ([f188909](https://github.com/n0-computer/noq/commit/f18890960d7911739b5ed9402e85e8f8ad02b834))

### ⛰️  Features

- *(quinn-udp)* Support illumos - ([e318cc4](https://github.com/n0-computer/noq/commit/e318cc4a80436fd9fa19c02886d682c49efca185))

### 🐛 Bug Fixes

- *(udp)* Use IPV6_PMTUDISC_PROBE instead of IP_PMTUDISC_PROBE on v6 - ([7551282](https://github.com/n0-computer/noq/commit/7551282bdcffcf6ed57887d4eb41ffb2a4d88143))
- *(udp)* Propagate error on apple_fast - ([53e13f2](https://github.com/n0-computer/noq/commit/53e13f2eb9f536713a82107d72175d800709d6fd))
- *(udp)* Retry on ErrorKind::Interrupted - ([31a0440](https://github.com/n0-computer/noq/commit/31a0440009afd5a7e29101410aa9d3da2d1f8077))
- *(udp)* Do not enable URO on Windows on ARM - ([7260987](https://github.com/n0-computer/noq/commit/7260987c91aa4fd9135b7eba3082f0be5cd9e8e6))

### 📚 Documentation

- Correct MSRV in README - ([a4c886c](https://github.com/n0-computer/noq/commit/a4c886c38a6e78916f683c01043b37b6d3a597cf))

### ⚙️ Miscellaneous Tasks

- *(ci)* Remove workaround for broken `cc` version - ([a55c114](https://github.com/n0-computer/noq/commit/a55c1141e96809a94fdafc131d51642c5444ed30))
- *(deps)* Bump codecov/codecov-action from 4 to 5 - ([3a9d176](https://github.com/n0-computer/noq/commit/3a9d176a7a131a1f6d9472c1a23fccdcb1275b52))
- *(udp)* Increase crate patch version to v0.5.8 - ([204b147](https://github.com/n0-computer/noq/commit/204b14792b5e92eb2c43cdb1ff05426412ff4466))
- Fix `cargo clippy` issues - ([f8b8c50](https://github.com/n0-computer/noq/commit/f8b8c5032e0db9d7dbc7c3452f09c7d1e2a4295d))

### Proto

- Split config module - ([1c463ab](https://github.com/n0-computer/noq/commit/1c463ab5b46d549c4e2b76fbaad9ddf50bac46bc))

### Quinn

- Avoid FIPS in docs.rs builds - ([37355ec](https://github.com/n0-computer/noq/commit/37355ec5e7da09435e99d4a35df7ffd70d410061))

## [quinn-proto-0.11.9](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.7..quinn-proto-0.11.9) - 2024-11-13

### ⛰️  Features

- *(quinn-proto)* Allow notifying of network path changes - ([4974621](https://github.com/n0-computer/noq/commit/497462129e2cd591347c89f7522640ab8aa6c70d))
- *(quinn-proto)* Support & test `wasm32-unknown-unknown` target - ([a0d8985](https://github.com/n0-computer/noq/commit/a0d8985021cfd45665da38f17376ba335fd44bb4))
- Enable rustls logging, gated by rustls-log feature flag - ([9be256e](https://github.com/n0-computer/noq/commit/9be256e1c48ad7a5d893079acda43c8fc9caede6))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update thiserror requirement from 1.0.21 to 2.0.3 - ([18b7956](https://github.com/n0-computer/noq/commit/18b79569693ea9d78ea127932f6d6e663664147f))

### CI

- Add test for netbsd - ([d23e4e4](https://github.com/n0-computer/noq/commit/d23e4e494f7446e21184bf58acd17a861ae73bba))

### Proto

- Rename frame::Type to FrameType - ([8c66491](https://github.com/n0-computer/noq/commit/8c664916f7b6718848eb43827b349472cfbe3213))
- Fix missing re-exports - ([7944e0f](https://github.com/n0-computer/noq/commit/7944e0fabcffe9c0d14f00d8eaa147f94f5970c7))
- Bump version to 0.11.9 - ([2a8b904](https://github.com/n0-computer/noq/commit/2a8b9044cc1a7108b63ff42746023bfbfec334bb))

### Quinn

- Fix missing re-exports - ([eebccff](https://github.com/n0-computer/noq/commit/eebccff309cb342c2faac3ea875ca81734685821))
- Bump version to 0.11.6 - ([66546dd](https://github.com/n0-computer/noq/commit/66546ddd5aee10672e31bb166e57891a13863171))

## [quinn-udp-0.5.7](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.6..quinn-udp-0.5.7) - 2024-11-05

### ⚙️ Miscellaneous Tasks

- *(deps)* Update rustls-platform-verifier requirement from 0.3 to 0.4 - ([c3e70aa](https://github.com/n0-computer/noq/commit/c3e70aa7ab9c51d8d976c3ea740641d9ac09dd91))

### Quinn-udp

- Increase crate patch version to v0.5.7 - ([a0bcb35](https://github.com/n0-computer/noq/commit/a0bcb35334686d6af2c23c27d9885e9750f91376))

## [quinn-udp-0.5.6](https://github.com/n0-computer/noq/compare/iroh-quinn-proto-0.12.0..quinn-udp-0.5.6) - 2024-10-28

### #2008

- Make max_idle_timeout negotiation commutative - ([31a95ee](https://github.com/n0-computer/noq/commit/31a95ee85fff18e2d937a99b84948a5bf6bec8df))

### ⛰️  Features

- *(quinn, quinn-proto)* Add aws-lc-rs-fips feature flag - ([aae5bdc](https://github.com/n0-computer/noq/commit/aae5bdc3fa9329748ac8b0cec846784c688f373c))
- *(quinn-udp)* Support both windows-sys v0.52 and v0.59 - ([a461695](https://github.com/n0-computer/noq/commit/a461695fe3bb20fa1e352f646a9678d07fb5d45a))
- *(udp/bench)* Support recvmmsg - ([91a639f](https://github.com/n0-computer/noq/commit/91a639f67c7ab2d7dbfd87932edcf2394340576f))
- Faster UDP/IO on Apple platforms - ([adc4a06](https://github.com/n0-computer/noq/commit/adc4a0684105dfefa31356e531e6c02d7e1a5c53))

### 🐛 Bug Fixes

- *(udp/bench)* Enforce max 64k UDP datagram limit - ([b5902da](https://github.com/n0-computer/noq/commit/b5902da5a95e863dfad7e1d15afaef07fc6fba0a))

### 🚜 Refactor

- *(udp/bench)* Add fn new_socket - ([a5e3b6f](https://github.com/n0-computer/noq/commit/a5e3b6f063e59e4331711477f7f308f0b0aa97f8))
- *(udp/bench)* Switch to async - ([a5046ad](https://github.com/n0-computer/noq/commit/a5046add78957bec4849fac366a00751f7ea5b70))

### ⚙️ Miscellaneous Tasks

- *(quinn-udp)* Increase crate patch version to v0.5.6 - ([e7ae563](https://github.com/n0-computer/noq/commit/e7ae56300a2782fa7b8a87821432d4cdce19791a))
- *(workflows/features)* Only test FIPS features on Ubuntu - ([459322b](https://github.com/n0-computer/noq/commit/459322b1800f7ae5612a6b4b890c5cd1b6a499bf))
- Add project tracking ([#24](https://github.com/n0-computer/noq/issues/24)) - ([fd9f5c1](https://github.com/n0-computer/noq/commit/fd9f5c10244caf14d1e49d610849c38e3756513a))
- Check coverage on multiple platforms - ([19a5e9d](https://github.com/n0-computer/noq/commit/19a5e9dfd0594971856c45b62b365738ab1adf22))
- Replace IP strings with address types - ([15a4dce](https://github.com/n0-computer/noq/commit/15a4dcef42bf10c84535ec7e8331db9e97918856))
- Test-run benchmarks - ([c7a8758](https://github.com/n0-computer/noq/commit/c7a8758ab9639412b36fc43455ff1288526a58cd))
- `cargo +nightly clippy --fix` - ([5dd3497](https://github.com/n0-computer/noq/commit/5dd3497107e97b6341eb519f080fd13907f26855))
- Expose negotiated_cipher_suite in the hadshake data - ([a5d9bd1](https://github.com/n0-computer/noq/commit/a5d9bd1154b7644ff22b75191a89db9687546fdb))

### PendingStreams

- Add missing internal API methods - ([62f1818](https://github.com/n0-computer/noq/commit/62f1818dc4b0377d8e646edc384583e7292a055c))
- Add alternative (unfair) send stream scheduling strategy - ([9d63e62](https://github.com/n0-computer/noq/commit/9d63e6236be5e831119ad6adb1de88b20bd93f5c))

### StreamState

- Allow reusing Recv instances - ([41850c8](https://github.com/n0-computer/noq/commit/41850c8a304f09c7d009a6e70e48f35bd737e1b5))

### Proto

- Remove unnecessary feature guard - ([983920f](https://github.com/n0-computer/noq/commit/983920f9627aa103e9d99dc5b78399a9706f1c96))
- Abstract more over ring dependency - ([425f147](https://github.com/n0-computer/noq/commit/425f14789925df51e328bfce6b9dab4a32199c2b))
- Export `ShouldTransmit` - ([41989fe](https://github.com/n0-computer/noq/commit/41989fef33738d281b1ca72801adf7137189aeff))
- Remove panic-on-drop from `Chunks` - ([bcb962b](https://github.com/n0-computer/noq/commit/bcb962b222f7c15fc8d8b27285eb9cf3bf689e80))
- Update DatagramState::outgoing_total on drop_oversized() - ([ead9b93](https://github.com/n0-computer/noq/commit/ead9b9316c155073c0984a243aeb9b84c5465298))

### Quinn

- Export endpoint::EndpointStats - ([43a9d76](https://github.com/n0-computer/noq/commit/43a9d768bedfd81bf87ca25ff11c7a3b091c4956))

## [iroh-quinn-proto-0.12.0](https://github.com/n0-computer/noq/compare/iroh-quinn-udp-0.5.5..iroh-quinn-proto-0.12.0) - 2024-11-04

### ⚙️ Miscellaneous Tasks

- Release 0.12.0 quinn and quinn-proto - ([4332156](https://github.com/n0-computer/noq/commit/43321560144932baebcb575796c36968951fc6d4))

## [iroh-quinn-udp-0.5.5](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.5..iroh-quinn-udp-0.5.5) - 2024-11-04

### ⛰️  Features

- QUIC Address discovery extension ([#12](https://github.com/n0-computer/noq/issues/12)) - ([94e970d](https://github.com/n0-computer/noq/commit/94e970d4d64cd0add28fbfd7f333922adc8f7b8b))

## [quinn-udp-0.5.5](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.8..quinn-udp-0.5.5) - 2024-09-03

### ⚙️ Miscellaneous Tasks

- *(quinn-udp)* Increase crate patch version to v0.5.5 - ([8bdbf42](https://github.com/n0-computer/noq/commit/8bdbf42a54f04b3bd2965d6ad0e2ce3966287330))

## [quinn-proto-0.11.8](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.7..quinn-proto-0.11.8) - 2024-09-03

### Proto

- Avoid panicking on rustls server config errors - ([a8ec510](https://github.com/n0-computer/noq/commit/a8ec510fd171380a50bd9b99f20a772980aabe47))
- Bump version to 0.11.8 for release ([#1981](https://github.com/n0-computer/noq/issues/1981)) - ([7c09b02](https://github.com/n0-computer/noq/commit/7c09b02073783830abb7304fc4642c5452cc6853))

## [quinn-proto-0.11.7](https://github.com/n0-computer/noq/compare/iroh-quinn-udp-0.5.4..quinn-proto-0.11.7) - 2024-09-02

### 🐛 Bug Fixes

- *(.github/android)* Pass matrix.target and increase api to v26 - ([5e5cc93](https://github.com/n0-computer/noq/commit/5e5cc936450e7a843f88ed4008d5df9374fb7dd8))
- *(.github/android)* Use API level 26 - ([bb02a12](https://github.com/n0-computer/noq/commit/bb02a12a8435a7732a1d762783eeacbb7e50418e))
- *(udp)* Typo in sendmsg error log - ([cef42cc](https://github.com/n0-computer/noq/commit/cef42cccef6fb6f02527ae4b2f42d7f7da878f62))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump android-actions/setup-android from 2 to 3 - ([abaa2d3](https://github.com/n0-computer/noq/commit/abaa2d3b1390975e20911199d20131ba629db50b))
- *(deps)* Bump actions/setup-java from 3 to 4 - ([1e48a70](https://github.com/n0-computer/noq/commit/1e48a703d5a7d7c7594acca2068cd6bd68e224c5))
- *(rust.yml)* Add workflow testing feature permutations - ([edf16a6](https://github.com/n0-computer/noq/commit/edf16a6f106379681509f229b6e45539fa3eebdb))
- Add Android job - ([1e00247](https://github.com/n0-computer/noq/commit/1e00247360779599eab4093897e332eb1ededf32))

## [iroh-quinn-udp-0.5.4](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.6..iroh-quinn-udp-0.5.4) - 2024-08-15

### ⚙️ Miscellaneous Tasks

- *(deps)* Update windows-sys requirement from 0.52 to 0.59 - ([91be546](https://github.com/n0-computer/noq/commit/91be5467387ebbabffa884f6abb1b7663c8ffec4))

## [quinn-proto-0.11.6](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.5..quinn-proto-0.11.6) - 2024-08-04

### Proto

- Bump version to 0.11.6 - ([2d06eef](https://github.com/n0-computer/noq/commit/2d06eef43fec927b0cf8f960bedb814bf3e4cc79))

## [quinn-proto-0.11.5](https://github.com/n0-computer/noq/compare/quinn-proto-0.11.4..quinn-proto-0.11.5) - 2024-08-04

### Proto

- Bump version to 0.11.5 - ([91b5a56](https://github.com/n0-computer/noq/commit/91b5a56424d23c1ad43263ccc9d1c81e9080d60d))

## [quinn-proto-0.11.4](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.4..quinn-proto-0.11.4) - 2024-08-02

### 🚜 Refactor

- *(udp)* Introduce log facade - ([244b44d](https://github.com/n0-computer/noq/commit/244b44d8cf790879588615d2cb347b59e18f0b4c))
- Move rust-version to workspace Cargo.toml - ([ce97879](https://github.com/n0-computer/noq/commit/ce97879e8d44e4b109efb08e88d1f3195d2c1770))

### ⚙️ Miscellaneous Tasks

- *(quinn)* Feature flag socket2 imports - ([2de91cf](https://github.com/n0-computer/noq/commit/2de91cfd7f2d39a930afdbab454d526346fed693))
- Move common package data to workspace Cargo.toml - ([9dbaff0](https://github.com/n0-computer/noq/commit/9dbaff0ea1be4faedd3cbdfbcf7b388a386f7da3))

### Quinn

- Bump version to 0.11.3 - ([b3f1493](https://github.com/n0-computer/noq/commit/b3f149386f978195634f1aec1d48cd1b5db5df20))

## [quinn-udp-0.5.4](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.3..quinn-udp-0.5.4) - 2024-07-22

### 🐛 Bug Fixes

- *(udp)* Feature flag tracing in windows.rs - ([061a74f](https://github.com/n0-computer/noq/commit/061a74fb6ef67b12f78bc2a3cfc9906e54762eeb))

## [quinn-udp-0.5.3](https://github.com/n0-computer/noq/compare/quinn-udp-0.5.2..quinn-udp-0.5.3) - 2024-07-19

### 🐛 Bug Fixes

- Remove unused dependency tracing-attributes - ([8f3f824](https://github.com/n0-computer/noq/commit/8f3f8242c9a36b7bfb16ab4712a127599a097144))

### 🚜 Refactor

- *(udp)* Add use declaration for tracing debug and error - ([349dcd6](https://github.com/n0-computer/noq/commit/349dcd6017cd9b1b1bf07c08460f2d18a14663e9))
- Use workspace dependency for tracing and tracing-subscriber - ([9e2272a](https://github.com/n0-computer/noq/commit/9e2272a477a76fa9656f6caf427c039416999432))

### 📚 Documentation

- *(quinn)* Clarify effects of setting AckFrequencyConfig - ([11050d6](https://github.com/n0-computer/noq/commit/11050d6fe3a10c9509e7435b1ec3808e05ed4b00))
- *(quinn)* Revise SendStream.stopped docs comment - ([02ed621](https://github.com/n0-computer/noq/commit/02ed62142d60226c198dbbeb13ef6548d03fd922))
- Revise and add additionall 0-rtt doc comments - ([9366f5e](https://github.com/n0-computer/noq/commit/9366f5e80b9cd801a8deb4ec171cc15fd63b25da))
- Remove reference to sendmmsg - ([7c4cce1](https://github.com/n0-computer/noq/commit/7c4cce1370e1d5f366e9f23fffce0469257b1bc8))

### ⚙️ Miscellaneous Tasks

- *(udp)* Make tracing optional and add optional log - ([8712910](https://github.com/n0-computer/noq/commit/8712910a4c0276d3ab25b426cca1e1110bd863db))
- Make initial destination cid configurable - ([03fe15f](https://github.com/n0-computer/noq/commit/03fe15f99ef251a259146218afd2aca7b5e27aad))

### Bench

- *(udp)* Measure non-GSO & GSO on localhost ([#1915](https://github.com/n0-computer/noq/issues/1915)) - ([36407fe](https://github.com/n0-computer/noq/commit/36407fecc31a794fb790ff8955f404d4ef346b09))

### Proto

- Avoid overflow in handshake done statistic - ([f0fa66f](https://github.com/n0-computer/noq/commit/f0fa66f871b80b9d2d7075d76967c649aecc0b77))
- Bump version to 0.11.4 - ([f484d63](https://github.com/n0-computer/noq/commit/f484d633efeb532634a1d67698a918d3432b15cc))

### Quinn

- Introduce wake_all() helper - ([0273e0a](https://github.com/n0-computer/noq/commit/0273e0a7044631afcf7e416250b9bf5373481841))
- Introduce wake_stream() helper - ([70f5194](https://github.com/n0-computer/noq/commit/70f5194fc85e7915aeb7d0e35d9e0a7cd635fb03))
- Make `Endpoint::client` dual-stack V6 by default - ([693c9b7](https://github.com/n0-computer/noq/commit/693c9b7cfbf89c541ba99523237594499984ffed))

## [quinn-udp-0.5.2](https://github.com/n0-computer/noq/compare/0.11.2..quinn-udp-0.5.2) - 2024-06-07

### 📚 Documentation

- *(udp)* Replace AsRawFd and AsRawSocket with AsFd and AsSocket - ([c66f45e](https://github.com/n0-computer/noq/commit/c66f45e985f9c0098afaf25810eb007f5bb1ee35))

### Udp

- Bump version to 0.5.2 - ([f117a74](https://github.com/n0-computer/noq/commit/f117a7430c8674d73ea7ceeeaf7f3a6015ea7426))

## [0.11.2](https://github.com/n0-computer/noq/compare/0.11.1..0.11.2) - 2024-05-23

### ⚙️ Miscellaneous Tasks

- Add rustls constructors with explicit initial - ([690736c](https://github.com/n0-computer/noq/commit/690736cb2fa555fa34ced24479688a90248d44a1))
- Introduce ConnectionIdParser - ([ee1c0fd](https://github.com/n0-computer/noq/commit/ee1c0fd143df3b6c2e8524ccc6b4dacc88a223f5))

### Proto

- Support creating config wrappers from Arc-wrapped configs - ([8bd0600](https://github.com/n0-computer/noq/commit/8bd0600089fa8bcf333df4cad2e4cac23b514a99))
- Make NoInitialCipherSuite Clone - ([f82beab](https://github.com/n0-computer/noq/commit/f82beab2f3d7cbed2e57a51864f115a9ce4a85d1))
- Make packet parsing APIs public - ([d9da98b](https://github.com/n0-computer/noq/commit/d9da98bdc83ff39f72de0b29acc358f3433c138f))
- Rename Plain types to Protected - ([6c9c252](https://github.com/n0-computer/noq/commit/6c9c252326534d21e1e484824f79ebed7ad5872b))

### Quinn

- Fix bytes read count in ReadExactError::FinishedEarly - ([f952714](https://github.com/n0-computer/noq/commit/f952714dfec3c2495ec3379fe23d4d4a5fede321))
- Return `ReadError::Reset` persistently - ([d38854b](https://github.com/n0-computer/noq/commit/d38854b0a6146c67e438ea140e609b2ce6165e39))
- Introduce RecvStream::received_reset - ([fc22ddd](https://github.com/n0-computer/noq/commit/fc22ddd7f865cec9750375a2cc48fe190685d3d4))

## [0.11.1](https://github.com/n0-computer/noq/compare/0.11.0..0.11.1) - 2024-05-08

### Quinn

- Alphabetize default features - ([1e54758](https://github.com/n0-computer/noq/commit/1e547588e8b3d86cfd6450cad73f480e1232c351))

## [0.11.0](https://github.com/n0-computer/noq/compare/perf-regression..0.11.0) - 2024-05-03

### #729

- Proto: write outgoing packets to caller-supplied memory ([#1697](https://github.com/n0-computer/noq/issues/1697)) - ([49aa4b6](https://github.com/n0-computer/noq/commit/49aa4b61e0a7dce07535eb8a288ecc3930afe2ef))

### 🐛 Bug Fixes

- *(quinn-udp)* Use TOS for IPv4-mapped IPv6 dst addrs - ([a947962](https://github.com/n0-computer/noq/commit/a947962131aba8a6521253d03cc948b20098a2d6))

### 🚜 Refactor

- *(endpoint)* Use array::from_fn instead of unsafe MaybeUninit - ([65bddc9](https://github.com/n0-computer/noq/commit/65bddc90187a93b2172519c72fc611258d0b2fd3))

### ⚡ Performance

- Change throughput units from MiB/s into Mb/s. - ([90118e7](https://github.com/n0-computer/noq/commit/90118e76b3340a3b8f0f6877f27eebde7315fea0))
- Hoist config construction out of conditionals - ([f0d1a45](https://github.com/n0-computer/noq/commit/f0d1a45639e2b89963e6d2b92ddc87fa7ac336ce))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump codecov/codecov-action from 3 to 4 - ([dcc8048](https://github.com/n0-computer/noq/commit/dcc8048974ce9b1ca6b365019149b5586ed88f4a))
- *(deps)* Bump peaceiris/actions-mdbook from 1 to 2 - ([b469e1c](https://github.com/n0-computer/noq/commit/b469e1c7ad7815df3f9d94335d6c454cd07412fa))
- *(deps)* Bump peaceiris/actions-gh-pages from 3 to 4 - ([52c285d](https://github.com/n0-computer/noq/commit/52c285d60f4c3282578ba63a849689c5ef875632))
- Make now explicit for Endpoint::connect() - ([307d80b](https://github.com/n0-computer/noq/commit/307d80b9398d4e1e305c0131f2c3989090ec9432))
- Refactor Endpoint to use Incoming - ([8311124](https://github.com/n0-computer/noq/commit/83111249e829a2f367e15376b207d787473b88c2))
- Pass codecov token explicitly - ([b570714](https://github.com/n0-computer/noq/commit/b5707140d5abd08dcdc182e8759bc4e577983d67))
- Guard rustls-specific types - ([7af5296](https://github.com/n0-computer/noq/commit/7af5296dc3078994b1567bef3afde62dddb1cea8))

### Proto

- Allow GSO to be manually disabled - ([a06838a](https://github.com/n0-computer/noq/commit/a06838abde23bbd64d9f527c85b34a6da69055aa))
- Allow test code to opt out of skipping packet numbers - ([bef7249](https://github.com/n0-computer/noq/commit/bef724969cb3568e99e291a969eb9b717aa6680f))
- Use deterministic packet numbers in tests that count ACKs - ([f07a40d](https://github.com/n0-computer/noq/commit/f07a40d7f1da99253408fb1ab3db91eef3fe07e6))
- Fix double-boxing of `congestion::ControllerFactory` - ([33fa6bb](https://github.com/n0-computer/noq/commit/33fa6bb24d298d6037d0ecd2162eba5ee3a85dd6))
- Add forgotten fields to Debug for TransportConfig - ([8c58cc7](https://github.com/n0-computer/noq/commit/8c58cc77815f054f3b4c6a2a5cd3bef3cab07fed))
- Don't panic when draining a unknown connection - ([394ac8c](https://github.com/n0-computer/noq/commit/394ac8c2b84497bb490659683ffd2f922ced8a0a))
- Detect stateless resets in authed and unprotected packets - ([7f26029](https://github.com/n0-computer/noq/commit/7f260292848a93d615eb43e6e88114a97e64daf1))
- Move IterErr below users - ([9f437c0](https://github.com/n0-computer/noq/commit/9f437c0da7491075ecef8beb2b5bcd2e3d5c4200))
- Yield transport error for Initial packets with no CRYPTO - ([470b213](https://github.com/n0-computer/noq/commit/470b2134c4cb54c18f6ae858de2a25005a97c255))
- Factor out Endpoint::retry - ([a9c4dbf](https://github.com/n0-computer/noq/commit/a9c4dbf91eb36cf3912851b51671b958c20cbfff))
- Remove the Side argument from ServerConfig::initial_keys() - ([85351bc](https://github.com/n0-computer/noq/commit/85351bc3999888d8abb124c0200dc2cb5a5f33b5))
- Rename InvalidDnsName to InvalidServerName - ([b61d9ec](https://github.com/n0-computer/noq/commit/b61d9ec5746317ae0ec5b827d6855d45de18d148))
- Deduplicate rustls ClientConfig setup - ([07e4281](https://github.com/n0-computer/noq/commit/07e428169bae3527f9c956f26d9c97a4c780430c))
- Add test helpers for custom ALPN crypto configs - ([285e1b6](https://github.com/n0-computer/noq/commit/285e1b650c8b8a687bcb9b4d6146045a16e860b4))
- Validate ClientConfig crypto provider - ([e6d4897](https://github.com/n0-computer/noq/commit/e6d48970afb76452204b3b7f748c8725aa864a66))
- Validate ServerConfig crypto provider - ([ce13559](https://github.com/n0-computer/noq/commit/ce135597786f8307db0336667636af2dbabe1e49))
- Factor out DatagramConnectionEvent - ([89f99bb](https://github.com/n0-computer/noq/commit/89f99bbdc0cc84baa8c9f3d3abfb667e127ef25d))
- Take advantage of rustls::quic::Suite being Copy - ([5b72270](https://github.com/n0-computer/noq/commit/5b722706b3cd46ce3f07fa2710b8a1024c7c6ed5))
- Remove incorrect feature guard - ([e764fe4](https://github.com/n0-computer/noq/commit/e764fe48cee11a6f10adfce85f899e39293c2cd9))

### Quinn

- Factor out TransmitState sub-struct from State - ([e6ee90c](https://github.com/n0-computer/noq/commit/e6ee90cb2be33d4a25e9e259a71aef91a24fba16))
- Add bounds in dyn Error types - ([e28b29f](https://github.com/n0-computer/noq/commit/e28b29f76ec7d830a029b9b8e17a684d98a2ec94))
- Use ClientConfig helper for tests - ([ae82c38](https://github.com/n0-computer/noq/commit/ae82c380dccf1549ca8287a147085ffffe03628b))
- Inline single-use helper function - ([7687540](https://github.com/n0-computer/noq/commit/76875408a9f18354334701a401228bd480b0b174))
- Allow rebinding an abstract socket - ([5beaf01](https://github.com/n0-computer/noq/commit/5beaf01793bd4b25738de783ebc62d2b20abe64f))
- Require rustls for insecure_connection example - ([faf7dbc](https://github.com/n0-computer/noq/commit/faf7dbc051f212a7329affdfec648c9c669d6224))
- UdpPoller::new() is only called if a runtime is enabled - ([74c0358](https://github.com/n0-computer/noq/commit/74c035822bd1ac53a65025b021b7d76768251c37))
- Add proper guards to Endpoint constructor helpers - ([272dd5d](https://github.com/n0-computer/noq/commit/272dd5d45f809ae42aa8cee25dbe896f389441de))

### Streams

- Extract max_send_data() helper - ([e1e9768](https://github.com/n0-computer/noq/commit/e1e9768bd47b0fde8da78f85b38ea8a2a40e564c))

### Udp

- Simplify socket state initialization - ([4f25f50](https://github.com/n0-computer/noq/commit/4f25f501ef4d009af9d3bef44d322c09c327b2df))
- Use set_socket_option_supported() wrapper - ([c02c8a5](https://github.com/n0-computer/noq/commit/c02c8a5a7a131c35be0e85dfe7d7e2a85c24a2b1))
- Don't log EMSGSIZE errors - ([5cca306](https://github.com/n0-computer/noq/commit/5cca3063f6f7747dcd9ec6e080ee48dcb5cfc4a7))
- Disable GSO on EINVAL - ([b3652a8](https://github.com/n0-computer/noq/commit/b3652a8336610fd969aa16ddd1488cf7b17d330b))
- Make cmsg a new module - ([5752e75](https://github.com/n0-computer/noq/commit/5752e75c92b343dc1ecce8bae52edb5a49d0475f))
- Preparation work to make cmsg Encoder / decode / Iter generic - ([ede912a](https://github.com/n0-computer/noq/commit/ede912a5777ddd554a9e4253877f3ccb34b40208))
- Move newly generic code so it can be reused - ([06630aa](https://github.com/n0-computer/noq/commit/06630aa025dee4a0a956d483c3fd625e0dde3f68))
- Add helper function to set option on windows socket - ([aa3b2e3](https://github.com/n0-computer/noq/commit/aa3b2e3e825e6414ef543ad666407cb5f9c7ebbd))
- Windows support for ECN and local addrs - ([8dfb63b](https://github.com/n0-computer/noq/commit/8dfb63b4c795fcdd828199ecedb5248094c7af12))
- Don't test setting ECN CE codepoint - ([1362483](https://github.com/n0-computer/noq/commit/136248365028a15d879b859c9e577e1dd6111ca2))
- Tolerate true IPv4 dest addrs when dual-stack - ([d2aae4d](https://github.com/n0-computer/noq/commit/d2aae4d6e7f8186b0762c96c7e09762fe3467ba5))
- Handle GRO in tests - ([7dc8edb](https://github.com/n0-computer/noq/commit/7dc8edb37e3bee18d83e147efb260b7eb0a6b4b9))
- Test GSO support - ([25c21a2](https://github.com/n0-computer/noq/commit/25c21a22975d67ab785e60fb44fb8f2637a4f5c5))
- Support GSO on Windows - ([33f6d89](https://github.com/n0-computer/noq/commit/33f6d89cf47fbd13083a465d6b044ada1b6099d2))
- Support GRO on Windows - ([2105122](https://github.com/n0-computer/noq/commit/21051222246e412e0094a42ba57d75303f64fcea))
- Make basic test work even if Ipv6 support is disabled - ([6e3d108](https://github.com/n0-computer/noq/commit/6e3d10857e724c749c37d29e2601140c26464858))
- Use io::Result<> where possible - ([20dff91](https://github.com/n0-computer/noq/commit/20dff915e1feaf293a739e68dc2c6ea2c6bbca09))
- Expand crate documentation - ([66cb4a9](https://github.com/n0-computer/noq/commit/66cb4a964a97bc0680498c4f8f5f67e5c65a848d))

## [0.10.1](https://github.com/n0-computer/noq/compare/0.10.0..0.10.1) - 2023-05-11

### 📚 Documentation

- Update the MSRV in the README - ([c0b9d42](https://github.com/n0-computer/noq/commit/c0b9d4233e45bfa08b562db0b6507545a86fd923))

### ⚙️ Miscellaneous Tasks

- Remove webpki dependency. - ([2f72a5b](https://github.com/n0-computer/noq/commit/2f72a5b8479cadb46a1ee6a00a71b173f5d5ed23))

## [0.10.0](https://github.com/n0-computer/noq/compare/0.9.3..0.10.0) - 2023-05-11

### ⛰️  Features

- Use BytesMut for Transmit content - ([89b527c](https://github.com/n0-computer/noq/commit/89b527c9a16f1985dd87b0bed8adfe78da430712))

### 🚜 Refactor

- Do not require &mut self in AsyncUdpSocket::poll_send - ([75524fc](https://github.com/n0-computer/noq/commit/75524fcb0bf9aee1f9a0c623edba7c108de67b28))

### 📚 Documentation

- *(quinn)* Remove restriction to tokio - ([c17315f](https://github.com/n0-computer/noq/commit/c17315fa105d3af215ee46730f7dd522c0022576))

### ⚡ Performance

- Specialize slice extension in Datagram::encode - ([d08ad01](https://github.com/n0-computer/noq/commit/d08ad01e4099024bfab82970251b1360698cef20))

### ⚙️ Miscellaneous Tasks

- Warn on unreachable_pub - ([134ef97](https://github.com/n0-computer/noq/commit/134ef97bdd499a11f6c708fd4de3e18959efb687))

### Deps

- Upgrade rustls v0.20.3 -> v0.21.0. - ([5d1f7bc](https://github.com/n0-computer/noq/commit/5d1f7bccf29e81d39a7b19bf395eb31d9ff905e0))

### Quinn

- Warn on unreachable_pub - ([4fd2df3](https://github.com/n0-computer/noq/commit/4fd2df30b045770c6627857276cd9755136be1a2))
- Take Arc<dyn Runtime> directly - ([3eb2636](https://github.com/n0-computer/noq/commit/3eb26361dba85f13b69e0eff6d934b28f70a37f8))

### Quinn-proto

- Bump version 0.9.3 -> 0.10.0. - ([b56d60b](https://github.com/n0-computer/noq/commit/b56d60bbec577d73e67abbba60ed389f0589f208))

### Udp

- Warn on unreachable_pub - ([eab8728](https://github.com/n0-computer/noq/commit/eab8728f055ac45efe19a86d3802024f26c45b0a))
- Avoid warning about unused set_sendmsg_einval() method - ([aaa58fc](https://github.com/n0-computer/noq/commit/aaa58fc501a63c010e82b1dfc50ceba302f6ec5a))
- Improve fragmentation suppression on *nix - ([23b1416](https://github.com/n0-computer/noq/commit/23b1416a0109b3121b53ed9d134348e73bf8abd3))
- Expose whether IP_DONTFRAG semantics apply - ([f4384e6](https://github.com/n0-computer/noq/commit/f4384e6edb02958d9f5b1c764cf61bd680cb32b1))

## [0.9.3](https://github.com/n0-computer/noq/compare/0.9.2..0.9.3) - 2022-12-05

### Connection

- Wake 'stopped' streams on stream finish events - ([1122c62](https://github.com/n0-computer/noq/commit/1122c627c35241eda2e87a9637d3bd5ea19f290c))

### Recv-stream

- Clean up any previously register wakers when RecvStream is dropped - ([70ef503](https://github.com/n0-computer/noq/commit/70ef5039e9ddba659e69801e1b4740333ea61189))

### Send-stream

- Unregister waker when Stopped is dropped - ([7ba0acb](https://github.com/n0-computer/noq/commit/7ba0acb8da407fbd6a6910a73252381d847c704f))
- Clean up any previously register wakers when SendStream is dropped - ([f6ae67e](https://github.com/n0-computer/noq/commit/f6ae67e2faa88a833a2b323f5d13f79ef5d2a052))
- Rely on cleaning up waker for Stopped in SendStream Drop impl - ([9f50319](https://github.com/n0-computer/noq/commit/9f503194218fe796a486767f7881dc47c793e3e2))

### Udp

- Add safe wrapper for setsockopt() - ([fd845b0](https://github.com/n0-computer/noq/commit/fd845b0c64c5ae6fdf9080ec11c263d23912c33f))

## [0.9.2](https://github.com/n0-computer/noq/compare/0.9.1..0.9.2) - 2022-11-29

### 🐛 Bug Fixes

- Don't bail if setting IP_RECVTOS fails - ([b8b9bff](https://github.com/n0-computer/noq/commit/b8b9bffe3c3e914c2f72dd5b815d113e093217ac))

## [0.9.0](https://github.com/n0-computer/noq/compare/udp-0.2.0..0.9.0) - 2022-10-28

### Quinn

- Bump dependency on tokio to 1.13 - ([28a2c80](https://github.com/n0-computer/noq/commit/28a2c8052ce5fa2abbd4ce385f6ee2f50cbfb770))

### Quinn-udp

- Bump version to 0.3 - ([57bd764](https://github.com/n0-computer/noq/commit/57bd7643e75c0e974acaa6d47967cf9c6c11cff8))

## [udp-0.2.0](https://github.com/n0-computer/noq/compare/0.8.0..udp-0.2.0) - 2022-09-17

### 🐛 Bug Fixes

- Read PEM certificates/keys by rustls_pemfile - ([02d6010](https://github.com/n0-computer/noq/commit/02d6010375996ad948afdb72b78879c2e4c76b26))

### 📚 Documentation

- Rm generic directory - ([6ceb3c6](https://github.com/n0-computer/noq/commit/6ceb3c63bb19d1b8c66b527c2fdc52053480d81d))
- Modify rustls ServerCertVerifier link - ([412a477](https://github.com/n0-computer/noq/commit/412a4775f3382c511e67b56f144946c857c8c86f))
- Use automatic links for urls - ([8fbbf33](https://github.com/n0-computer/noq/commit/8fbbf33440c07b1b9452132a0127cd5b96dc8bb9))
- Fix broken item links - ([c9e1012](https://github.com/n0-computer/noq/commit/c9e10128852e448fe85ecb88ca8f60135c13d678))
- Match MSRV to 1.53 in readme - ([ac56221](https://github.com/n0-computer/noq/commit/ac562218601af99b11bf4044818defa21b445e3a))
- Update the client certificates example to a working config ([#1328](https://github.com/n0-computer/noq/issues/1328)) - ([e10075c](https://github.com/n0-computer/noq/commit/e10075cf2fdb0dcca62a79291929369e95e84c86))
- Add/modify docs - ([3a25582](https://github.com/n0-computer/noq/commit/3a2558258034e60989bbc199d4d8b0b7297ee269))

### ⚙️ Miscellaneous Tasks

- Check private docs for links as well - ([8dca9fc](https://github.com/n0-computer/noq/commit/8dca9fcc37e819add3e96d6f7965a2b61897f582))

### Bbr

- Apply clippy suggestions to avoid unnecessary late initialization - ([92ab452](https://github.com/n0-computer/noq/commit/92ab452e1b573e5f9bf7736060b0318b8f07a813))
- Avoid unwrapping a value we just set - ([a87b326](https://github.com/n0-computer/noq/commit/a87b3262ff7daeac3a76857d1eaaf944d5cd9d29))
- Avoid unwrapping checked Option value - ([4630670](https://github.com/n0-computer/noq/commit/4630670655ce568813689530b7e579fe53d38145))
- Avoid unwrapping another checked Option value - ([8da9cf5](https://github.com/n0-computer/noq/commit/8da9cf55e5ec8b9390e41bb9eee3484b67be7cc7))
- Implement Default for MinMax - ([75b2b11](https://github.com/n0-computer/noq/commit/75b2b118ecdada612b296906fde94c6bf282ce6a))
- Derive default for AckAggregationState - ([60dd3da](https://github.com/n0-computer/noq/commit/60dd3da2a1f67526a3354dbc10a29ee8998e593c))
- Change sent_time type to Instant - ([5e0df6c](https://github.com/n0-computer/noq/commit/5e0df6c1f1668cf35ab14448b97b4b128be3cbdd))
- Reorder code according to prevailing style - ([c0b50b4](https://github.com/n0-computer/noq/commit/c0b50b4a0dd72c8dc7c651c404975728ce420383))

### Book

- Clean up example certificate code - ([2bf23d6](https://github.com/n0-computer/noq/commit/2bf23d6b330700110741f344853d72553782512e))
- Clean up whitespace - ([cd00119](https://github.com/n0-computer/noq/commit/cd00119d254f9442618a7f1b8f748dcb9f309740))

### Config

- Add ServerConfig::transport_config() builder method - ([d522b6d](https://github.com/n0-computer/noq/commit/d522b6dd63a88b5bf097addfc26f0d2ad35a367b))
- Make ClientConfig fields private - ([838ad7c](https://github.com/n0-computer/noq/commit/838ad7c4715f032196449bbc5f6d367a9aaa951b))

### Connection

- Change overly verbose info span to debug - ([dfa4f0e](https://github.com/n0-computer/noq/commit/dfa4f0e296479ed204c26eda98640790bcdb298a))

### Udp

- Silence warnings on macOS - ([0db9064](https://github.com/n0-computer/noq/commit/0db9064d062547452d3d7e7920c7f0ed24a95c23))

## [0.8.0](https://github.com/n0-computer/noq/compare/0.7.0..0.8.0) - 2021-11-14

### ⛰️  Features

- *(cca)* Cubic ([#1122](https://github.com/n0-computer/noq/issues/1122)) - ([3f908a2](https://github.com/n0-computer/noq/commit/3f908a2c8c1ec4585212d776fafe536ea17bf2b4))

### ⚡ Performance

- Adopt more convential crate layout - ([85dde10](https://github.com/n0-computer/noq/commit/85dde101bd7310fee784030039fabee019417a17))
- Tweak style in bind_socket() - ([0f285bd](https://github.com/n0-computer/noq/commit/0f285bd751b08a3de5c6b299fbc1738877b2f4a4))
- Use dual stack socket for endpoints - ([2870519](https://github.com/n0-computer/noq/commit/2870519f6eb27e13f8597bc4d5a8b49fcae3425d))

### Chore

- Remove unused import - ([858a26a](https://github.com/n0-computer/noq/commit/858a26a6c6f861b33d5b28dfd5c679bd7d46b910))
- Disable unused default features for various crates - ([60b9f9f](https://github.com/n0-computer/noq/commit/60b9f9ff70431fa8da7ec073fe7fc47b3c854cda))

### Crypto

- Return Option from next_1rtt_keys() - ([e07835b](https://github.com/n0-computer/noq/commit/e07835b954d6c8653b488e82c167b09cdf594573))

### Endpoint

- Allow override server configuration - ([9bb4971](https://github.com/n0-computer/noq/commit/9bb4971b8d2b36fba97fd9b03b5d24940a2ad920))

### Followup

- Rename "stateless retry" -> "retry" - ([25d9a40](https://github.com/n0-computer/noq/commit/25d9a40bf97b020661659d752501c3597a65deca))

### Interop

- Remove H3 support - ([53b063b](https://github.com/n0-computer/noq/commit/53b063b9cdc6e671f2e87ab8b1d5bd2da1870a56))

### Quinn

- Only depend on rt-multi-thread as a dev-dependency - ([7f1aa1e](https://github.com/n0-computer/noq/commit/7f1aa1ead3dc02f32e0f2be9afbe9b6ac65bfbcb))

### Quinn-udp

- Normalize Cargo.toml formatting - ([b65a402](https://github.com/n0-computer/noq/commit/b65a4026349da256138ea4819a8b887a3b1ee9b2))
- Bump version number - ([91d22f7](https://github.com/n0-computer/noq/commit/91d22f73a65a93888533d460a04159c6504a0964))

## [0.7.0](https://github.com/n0-computer/noq/compare/0.6.1..0.7.0) - 2021-03-02

### ⚡ Performance

- Use tokio::try_join instead of select - ([1203960](https://github.com/n0-computer/noq/commit/12039602ae6d91d1361acb4d9b2ad11df2bbaed8))

### ⚙️ Miscellaneous Tasks

- Tracing spans: peer | alpn | test - ([3589dbf](https://github.com/n0-computer/noq/commit/3589dbf7871f407d286d54c8f00897b953412fbe))
- Enable tracing by default in tests - ([77aeb55](https://github.com/n0-computer/noq/commit/77aeb5544ba5722ba09fd89e0bc4ccd00cca3994))
- Copy tracing subscriber setup from quinn - ([48a3213](https://github.com/n0-computer/noq/commit/48a3213f74e08684457e27aa13694fed836c807b))
- Move UdpExt functionality into platform-specific UdpSocket types - ([22fa31d](https://github.com/n0-computer/noq/commit/22fa31d571d13c5a513ff51c690d83f3f2896837))
- Keep stopped state in Recv - ([90e903e](https://github.com/n0-computer/noq/commit/90e903e3156824c13c92fd2829067f9d9662afb4))
- Provide datagrams API access through special-purpose type - ([971265d](https://github.com/n0-computer/noq/commit/971265d9ffb97a1e86a99ff061564814ecb365ca))
- Prefer more efficient cipher suites - ([3de2727](https://github.com/n0-computer/noq/commit/3de2727b94de4755b9d67a40bca146cbf1652b8e))

### Fuzzing

- Adds target for streams type. - ([c054fb3](https://github.com/n0-computer/noq/commit/c054fb36cbcf435607419e58846f89138768ce94))

### H3

- Remove unused Server struct - ([88a0796](https://github.com/n0-computer/noq/commit/88a0796c0a79f6ab49542604112679c4135c778e))
- Reorganize public API - ([a6ff500](https://github.com/n0-computer/noq/commit/a6ff500f6f3274ec7e792f112088538ac68dfce9))
- Reorder server structs and methods - ([7cd10d1](https://github.com/n0-computer/noq/commit/7cd10d16b36eb93e1acc20ad20b55dd701526ec6))
- Fix request cancellation error code - ([158837e](https://github.com/n0-computer/noq/commit/158837e86a27428b3ec61bda7b4033a58074c6c9))
- Document server module - ([e440425](https://github.com/n0-computer/noq/commit/e4404259476a1e75605c4d2ea397d6d3b848d99a))
- Reorder client API - ([f3d7b03](https://github.com/n0-computer/noq/commit/f3d7b03fa1f8d3e13cef18443fed9688f06f2bcd))
- Add a shortchut to build a default client - ([a55712d](https://github.com/n0-computer/noq/commit/a55712d58c773dc8c774e7cb731390dc1c5db557))
- Client documentation - ([c2d6d71](https://github.com/n0-computer/noq/commit/c2d6d716e2700405e87ba0d96491dda1a8efc7ce))
- Body documentation - ([34654bc](https://github.com/n0-computer/noq/commit/34654bc630cbfba848b232832a785d9a47998a85))
- Remove unused error helper - ([e212233](https://github.com/n0-computer/noq/commit/e2122331167cf31ab5e6d193b0a0aef1a4b7c9bd))
- Remove error code - ([8ee24df](https://github.com/n0-computer/noq/commit/8ee24df4d6b00ef5a1478126df2d856cde93e569))
- Settings and errors documentation - ([28e211c](https://github.com/n0-computer/noq/commit/28e211cf4bfde26181e8fc9db65a9c128db92a62))
- Fix IO error wrongly wrapped into Error::Peer - ([bae7e19](https://github.com/n0-computer/noq/commit/bae7e199827ca406a9d43d0f4662ef6ce8a09379))
- Make client able wait for endpoint idle state - ([e9e973d](https://github.com/n0-computer/noq/commit/e9e973de2972afbfc991809ef6e33799a2738000))
- Rework the client example for clarity - ([8894e3e](https://github.com/n0-computer/noq/commit/8894e3ee960fdc24b34f7187a108c23a1e142165))
- Rework server example for clarity, remove helpers - ([b2351ef](https://github.com/n0-computer/noq/commit/b2351efc00fd34697e844b3cc38ab6d7f304aeeb))
- Documentation index - ([be998ac](https://github.com/n0-computer/noq/commit/be998ac9f367aba2daee82b1abe462aa320cc13f))
- Bench factorize server - ([739719f](https://github.com/n0-computer/noq/commit/739719f884721628342d7ab9b1a33f87e4d70c4b))
- Kill the server, fix bench - ([a1d538c](https://github.com/n0-computer/noq/commit/a1d538c7d3aba1a12ae00856cc5b09be93c8746e))
- Fix comment style - ([416cca0](https://github.com/n0-computer/noq/commit/416cca0935773013f6665393c7c45c1b7f2c91b8))
- Let the OS choose bench ports - ([690c29f](https://github.com/n0-computer/noq/commit/690c29f135609007026f072952be17f1e64d6fb9))
- Orthogonal bench server spawner - ([050b6fd](https://github.com/n0-computer/noq/commit/050b6fdc3f1627def4bf1af2f8b440f1259c86b9))
- Rename bench throughput -> download - ([d6c290a](https://github.com/n0-computer/noq/commit/d6c290a64142074038c9b67283469c312673b2e6))
- Upload benchmarks - ([963695d](https://github.com/n0-computer/noq/commit/963695db1d4a8caa73421af3be1e7f26c43851a9))
- Isolate throughput bench and helpers - ([f3cc676](https://github.com/n0-computer/noq/commit/f3cc67601fb422a5faa66207e692f58c1a06da1d))
- Build benchmark context with settings - ([10cde92](https://github.com/n0-computer/noq/commit/10cde920f60faf27720686f057239f638cff2777))
- Impl default for bench context - ([2aa424a](https://github.com/n0-computer/noq/commit/2aa424a3ce67714346d090c0be874fed395e827b))
- Request benchmarks - ([b524f6a](https://github.com/n0-computer/noq/commit/b524f6ac4c4f33812e8d7fe311bcf4474a4d41a5))
- Make payload-frames carry a Buf impl - ([90008bf](https://github.com/n0-computer/noq/commit/90008bf7fb4bb6e06318dbdc28ebc90577fff605))
- Create Error variant for Body errors - ([74d8798](https://github.com/n0-computer/noq/commit/74d8798b5d5ec189ba3262e7f7aeec9d4b71c874))
- Change structure of Body - ([8a97c8d](https://github.com/n0-computer/noq/commit/8a97c8d39776891aae7879c956abdcd65cd6cda7))
- Impl HttpBody for Body - ([ae4b61c](https://github.com/n0-computer/noq/commit/ae4b61cf1ec4b4fce5b7a29a4c6de23298b065fb))
- Make streams reset() take &mut, not ownership - ([60949bc](https://github.com/n0-computer/noq/commit/60949bc0df4f4091b48dbb65427568148fb4dc0d))
- Body stream helper for benches - ([57fe592](https://github.com/n0-computer/noq/commit/57fe592bd736f3e733f738b9c147e512b309efcd))
- HttpBody server integration in SendResponse - ([34c9f45](https://github.com/n0-computer/noq/commit/34c9f458020daa95c322180f22b66effa820ad65))
- Poll method for header decoding - ([36c4d0b](https://github.com/n0-computer/noq/commit/36c4d0ba53cb20ae24c25379f1a35ec57a70412d))
- Rewrite client to use SendData<HttpBody, _> - ([4484705](https://github.com/n0-computer/noq/commit/448470518a55a291107807a150cb57aa6a6b8e07))
- HttpBody implementation on the receive side - ([586cb59](https://github.com/n0-computer/noq/commit/586cb59a4e1d3252934fb630709b1ce1be802144))
- Restore canceling API - ([61332ec](https://github.com/n0-computer/noq/commit/61332ec52cc694fd6f2b31c1ba6b5250afc08c3b))
- Refactor header receiving code into RecvData - ([cf43135](https://github.com/n0-computer/noq/commit/cf431350ebfed41bef2b33d2f5461622ebcad131))
- Tweaks to error types - ([6df0b1d](https://github.com/n0-computer/noq/commit/6df0b1d6e15722af043d265969d2ec531d517f4c))
- Don't take ownership for request cancellation - ([99df1df](https://github.com/n0-computer/noq/commit/99df1dfbb1f77cdf977b180af8c08398cc5cc3fa))
- Update docs with HttpBody API - ([56192ff](https://github.com/n0-computer/noq/commit/56192ff8750e68abfa6b3f8871aab80a4711cc95))
- Use a HashSet for in-flight request tracking - ([1d59775](https://github.com/n0-computer/noq/commit/1d5977548a821667ff9bfdbf4971134a2a7b761c))
- Fix client response canceling - ([d45ce73](https://github.com/n0-computer/noq/commit/d45ce73660b6cc1cf01c51b179b2fc03ffedf69a))
- Test response canceling from server - ([b2ebb47](https://github.com/n0-computer/noq/commit/b2ebb47b8c9f3036ee4836ee1a9ad5ca662d8c3a))
- Send get requests with FakeRequest helpers - ([061733d](https://github.com/n0-computer/noq/commit/061733dc1e82fa3c89278c24a7dc1d22f5817015))
- Ignore unknown frames - ([5f5be44](https://github.com/n0-computer/noq/commit/5f5be440dc2e11de2cc907307c109027adb99199))
- Ignore unknown incoming uni stream - ([68b866b](https://github.com/n0-computer/noq/commit/68b866bc63084046b8e9e19e779516ed8066639c))
- Simplify ownership of SendStream (fixes #768) - ([aa1ebba](https://github.com/n0-computer/noq/commit/aa1ebbab7647e0f6b971a014cc45aa3496bac5f8))
- Poll for STOP_SENDING - ([cbbd76e](https://github.com/n0-computer/noq/commit/cbbd76ec608aafdcc3a64245f0ede710cb79bd27))
- Reject request when headers are invalid - ([cf0801c](https://github.com/n0-computer/noq/commit/cf0801c8fdaab8dd57149cbd090768f16b0165a6))
- Check authority validity for server - ([94d5de1](https://github.com/n0-computer/noq/commit/94d5de1709906967c0529258ea00b271f8972852))
- Trace arriving requests - ([2444a60](https://github.com/n0-computer/noq/commit/2444a60220a9a19f935e61486a323588e84709e3))
- Check request authority for client - ([448ba9a](https://github.com/n0-computer/noq/commit/448ba9a061a40fcc1a667f1fd749513c1cfd343f))
- Make the h3 client default port 443 - ([4945573](https://github.com/n0-computer/noq/commit/4945573fda9424ac30ed9b8c5a53b5a6ff2995d1))
- Ignore any number of unknown settings - ([0c6a27c](https://github.com/n0-computer/noq/commit/0c6a27c37ad66422a1e8356b68947dbccb7d82c9))
- Name pin projections as required by 0.4.21 - ([b5c8a21](https://github.com/n0-computer/noq/commit/b5c8a218ad9d72fdaf168046e471dffaa4ebea8f))
- Tests log level from env - ([b940a3a](https://github.com/n0-computer/noq/commit/b940a3ae363102f73d020267cc408c81ec556e16))
- Clarify connection end return value - ([878970b](https://github.com/n0-computer/noq/commit/878970b833a06ee4794aaacf7f1eb1a841d38135))
- GoAway from client - ([0cdf96e](https://github.com/n0-computer/noq/commit/0cdf96e92104efa8c41e2332ad241d6cc7a73b3c))
- Store side information in proto::connection - ([22200ae](https://github.com/n0-computer/noq/commit/22200ae79e424ca3a1268075fdb528f7be80e38e))
- Refactor GoAway mechanism to actually use the id - ([b851934](https://github.com/n0-computer/noq/commit/b8519342281c0c6c015d49c9949522d1ed509018))
- Prevent client to start new requests on shutdown - ([9a79718](https://github.com/n0-computer/noq/commit/9a797187791a092758c572a7c6084821da2c14f5))
- Refactor shutdown condition in h3 proto - ([9594ba7](https://github.com/n0-computer/noq/commit/9594ba7ee3556cb4c18a3a04317259f5d01c5766))
- Wake connection on request finish - ([babb07b](https://github.com/n0-computer/noq/commit/babb07b079e7e3ac4ff2fa7ef25b0dac5e934377))

### Perf

- Use owned buffers - ([312c0f0](https://github.com/n0-computer/noq/commit/312c0f041c1191b179fe5cd552a0c4c6d129226b))

### Certificate

- Accept pem format ([#829](https://github.com/n0-computer/noq/issues/829)) - ([2892490](https://github.com/n0-computer/noq/commit/2892490057e30587c089e158ce515d7b0eec5ada))

### Fuzz

- Change config syntax to allow merging - ([c4af9ec](https://github.com/n0-computer/noq/commit/c4af9ecb1c9352f80a407cbe92edca3fcba4dfca))

### Interop

- Missing short option for listen - ([897e1f3](https://github.com/n0-computer/noq/commit/897e1f3e07694eea708456a9d8067b450f340ca4))
- Remove stale comment - ([d1df33a](https://github.com/n0-computer/noq/commit/d1df33ab5fabd03cf36b054e787507c9f5f5aa25))
- Make h3_get() faster and return the size read - ([1c835f8](https://github.com/n0-computer/noq/commit/1c835f86882f9dc07c1f25b8a46536b56a5d597c))
- Hq get accepts a path - ([fdcf0d2](https://github.com/n0-computer/noq/commit/fdcf0d27ccd922314994b4f2c4e9ae4b7f87f13c))
- Client throughput test `T` - ([ec12aa8](https://github.com/n0-computer/noq/commit/ec12aa8be3042217d3cc88e47f7fb8c6846ffd73))
- Rename hq methods for symmetry - ([d506d82](https://github.com/n0-computer/noq/commit/d506d8286072478d22b2b7deaf3f8788bb36f7ae))
- Server throughput test `T` - ([005cf4e](https://github.com/n0-computer/noq/commit/005cf4e08190c33dc8158d801f190e31af8a0596))
- Beef up transport config so `T` passes - ([ab76cab](https://github.com/n0-computer/noq/commit/ab76cab2bad736d4a19f8644c99ef7812cfff3c3))
- H3 remembers decoding with a QPACK dyn ref - ([7f73546](https://github.com/n0-computer/noq/commit/7f7354666c8e714b1694c24ead2cba7d23057e65))
- Client `d` test - ([30c4e5f](https://github.com/n0-computer/noq/commit/30c4e5fcfbd48a6063527c7783c2ca50c626f034))
- Custom header for dyn encoding from server - ([48e2e80](https://github.com/n0-computer/noq/commit/48e2e80ac9a947cf2039eabe5da0d7a99706875b))
- Make h2 accept self-signed certs - ([92123dc](https://github.com/n0-computer/noq/commit/92123dc0c54690cf805fb58d1c4204ab9a03fa83))
- Make qif tool catch up qpack API - ([ea82372](https://github.com/n0-computer/noq/commit/ea8237209f7fff6856c656c570f655129282dbd7))
- Fix qif clippy warnings - ([a6398dd](https://github.com/n0-computer/noq/commit/a6398ddf1df7693f399cbff96f023db88a0361cc))
- Rewrite qif tool error management - ([b3f9288](https://github.com/n0-computer/noq/commit/b3f9288c2994d0c21585708c5a207f7df72c2347))
- Doc for qif tool - ([3ec3612](https://github.com/n0-computer/noq/commit/3ec36129829d4b871438de9aa942f38a5d63c4eb))
- Send Alt-Svc from h2 and h1 - ([8647b5c](https://github.com/n0-computer/noq/commit/8647b5c089b1c7c505a1bf7317642301929c561c))
- Remove type length limit after 1.47 release - ([d4ac405](https://github.com/n0-computer/noq/commit/d4ac4057bd25e93e3aa29f961ab560ad47443344))

### Proto

- Rename UnsupportedVersion fields - ([766d20a](https://github.com/n0-computer/noq/commit/766d20a59230845b5105a4b53bce26819ac6e600))
- Add more high-level API docs to Connection, closes #924 ([#926](https://github.com/n0-computer/noq/issues/926)) - ([cfe6570](https://github.com/n0-computer/noq/commit/cfe6570a66f669bfe7bd104f6f56b1d38132127c))

### Quinn

- Test export_keying_material - ([363b353](https://github.com/n0-computer/noq/commit/363b3539ac60bd21f9139df00ec8929a3481ba62))
- Print socket addresses in example client/server - ([4420b61](https://github.com/n0-computer/noq/commit/4420b61aaac7568905573b3d6650eefc9c14ff0c))
- Remove unused field RecvStream::any_data_read - ([6a58b3f](https://github.com/n0-computer/noq/commit/6a58b3f542af595d454abb2b3672d521c8b3cf20))
- Properly await client connection setup in benchmarks - ([8b8f640](https://github.com/n0-computer/noq/commit/8b8f6401bf7f3b99176adfe1380433ee2e59853b))
- Unify ordered and unordered read APIs - ([a280b77](https://github.com/n0-computer/noq/commit/a280b7770fe7a2e84a10ca837d6a3d92e90170ad))
- Split streams module in send/recv parts - ([14db885](https://github.com/n0-computer/noq/commit/14db88562de0efa86aa5bfe007dfe6b29306feaf))

### Quinn-h3

- Clarify error message for closed control stream - ([ea81e65](https://github.com/n0-computer/noq/commit/ea81e654da952527f60f699e76eef9a1712df4c7))
- Enable client-side key logging in tests - ([661884f](https://github.com/n0-computer/noq/commit/661884f1ca4ebca6be61242ca2211789525a0c76))
- Reduce rightward drift in RecvUni Future impl - ([6bbea44](https://github.com/n0-computer/noq/commit/6bbea44fb52147c3e218a72ea29e5288bcc1f5fd))
- Fix typo in example function name - ([6de0b47](https://github.com/n0-computer/noq/commit/6de0b470be967fecc76c505eb4087f666b0b1a8f))
- Improve trace output - ([889d2b3](https://github.com/n0-computer/noq/commit/889d2b3e034e19f79876a1a34d1b49ed983efea5))
- Change 4-tuple to a struct - ([0dd5537](https://github.com/n0-computer/noq/commit/0dd5537e255a66a7dee789446cf50b6dcf0056aa))
- Limit amount of data decoded (fixes #994) - ([30c09d5](https://github.com/n0-computer/noq/commit/30c09d5c082231103c6f93bf2dd4b8b506528618))
- Partially revert limiting decoded data - ([f5d53a1](https://github.com/n0-computer/noq/commit/f5d53a1cbd3324754da9fffc4473c76abd3d54f0))

### Quinn-proto

- Merge ExportKeyingMaterial trait into Session - ([bc1c1a7](https://github.com/n0-computer/noq/commit/bc1c1a7e0e699fb419d69338907d23903d0c9670))
- Tweak ordering in RetryToken - ([3f3335e](https://github.com/n0-computer/noq/commit/3f3335e2428f22bdd5a019879a9bce1e4c704c5b))
- Improve grouping in RetryToken impl - ([84ba340](https://github.com/n0-computer/noq/commit/84ba3406974afaec51aa97cbf09b1f357fe7c002))
- Remove RetryToken TODO comment - ([1e70959](https://github.com/n0-computer/noq/commit/1e7095941e67f8289060355641661f07e0c89964))
- Generalize over read methods - ([13f1169](https://github.com/n0-computer/noq/commit/13f1169286ec6c8f0aae86f66755a06f6e7fdac8))
- Read crypto stream as bytes - ([72e0f9a](https://github.com/n0-computer/noq/commit/72e0f9aa5a65786b790fe36c44378c8c9cbc1b81))
- Add max_length argument to Assembler::read_chunk() - ([ce67167](https://github.com/n0-computer/noq/commit/ce671679688cd49569182da7f16c4e2b7b89df8b))
- Remove slice-based read API from Assembler - ([0439ec5](https://github.com/n0-computer/noq/commit/0439ec529871abc620e1880a39371aa1571d266c))
- Rename Assembler::read_chunk() to read() - ([6e9db53](https://github.com/n0-computer/noq/commit/6e9db53d14e1bb5fa17ebf44ccf32e5a39ee6ff7))
- Split streams module up - ([6ce0ef2](https://github.com/n0-computer/noq/commit/6ce0ef2542674a5e6b0b667d2a40cb71dd534dd6))
- Split connection::streams::types into send and recv modules - ([7947ad5](https://github.com/n0-computer/noq/commit/7947ad5854ccaf9af0a815341ec83c1651b36fa7))
- Check for stopped assembler before reading data - ([f2d01fb](https://github.com/n0-computer/noq/commit/f2d01fb2ad0d466255ab978a00993a554717047c))
- Remove read() methods in favor of read_chunk() - ([ab98859](https://github.com/n0-computer/noq/commit/ab98859756cde1dd2d37305bfb03be4c2c9d7a30))
- Rename read_chunk() to read() - ([f569495](https://github.com/n0-computer/noq/commit/f569495b71bbf49ec1eb6a018c23ca8817ee5efc))
- Let Assembler take responsibility for reads from stopped streams - ([0a07eab](https://github.com/n0-computer/noq/commit/0a07eaba20890a89ea5bf332cc2a8a2e31ba05ef))
- Add missing defragmented decrement - ([39c4c28](https://github.com/n0-computer/noq/commit/39c4c2883bc71a6b7a2fc063e9a2025ceef66d8c))
- Move ShouldTransmit into streams module - ([1ac9da4](https://github.com/n0-computer/noq/commit/1ac9da4be4c32fed27b3c1e928bd004baa839b69))
- Simplify ShouldTransmit interface - ([672cbec](https://github.com/n0-computer/noq/commit/672cbec5e578b9d6c053ad568a57da0392d3590c))
- Merge add_read_credits() into post_read() - ([ab3b74f](https://github.com/n0-computer/noq/commit/ab3b74f62d1aeb93aefb93a15dc84b7adae5bd48))
- Move post_read() logic into Retransmits - ([31f1ecb](https://github.com/n0-computer/noq/commit/31f1ecb1f72f3ad46eb28242820cecddbacdd839))
- Unify ordered and unordered read paths in assembler - ([ae29bb6](https://github.com/n0-computer/noq/commit/ae29bb6c305400a1bb1b9de12bfda68fdf6ff241))
- Unify API for ordered and unordered reads - ([07db694](https://github.com/n0-computer/noq/commit/07db694a54c0395fa67c77cdead8369f0d3a4a0e))
- Rename assembler::Chunk to Buffer - ([5350f23](https://github.com/n0-computer/noq/commit/5350f23da17b11315c979c421e794792abcf9c31))
- Use struct to yield data from assembler - ([81ea06b](https://github.com/n0-computer/noq/commit/81ea06bf92711a81fa4aba138f8dd0164e50bc5b))
- Yield read data as Chunks - ([6a7f861](https://github.com/n0-computer/noq/commit/6a7f861a1ee95d2fb2469fd9b1323a4068738c9d))
- Move end from Assembler into Recv - ([dedcca1](https://github.com/n0-computer/noq/commit/dedcca1cff5edda56bc70b65bf9754303ba794b2))
- Move stream stopping logic into Recv - ([c29d9ac](https://github.com/n0-computer/noq/commit/c29d9ac5d8978eb7fb9a241ca066f5ef492930dc))
- In ordered mode, eagerly discard previously read data - ([2610577](https://github.com/n0-computer/noq/commit/261057786dbdd730223f7a71fca6c5cf3f73b182))
- Split ordering check out of read() path - ([3aca40b](https://github.com/n0-computer/noq/commit/3aca40b47f6102cd03ff82d11e4a6d0f62c49fd3))
- Deduplicate when entering unordered mode (fix #1009) - ([2687ef8](https://github.com/n0-computer/noq/commit/2687ef8df4f506c594fb7599bc2a91c2e74cc5f0))
- Trigger defragmentation based on over-allocation (fix #981) - ([b9eb42e](https://github.com/n0-computer/noq/commit/b9eb42ee75fa6b24a3798a33545968f8aa8f3488))
- Unpack logic for Connection::space_can_send() - ([34f910b](https://github.com/n0-computer/noq/commit/34f910bae626402bacb8dfa8cd0d5f04b1709ae9))
- Return early from finish_and_track_packet() - ([8630946](https://github.com/n0-computer/noq/commit/863094657120b63181e3f229af0ce820815fee35))
- Inline single-use method - ([095f402](https://github.com/n0-computer/noq/commit/095f402a9ff6539620a26a6d5fc44c71901a9d22))
- Remove unnecessary RecvState::Closed - ([dd23094](https://github.com/n0-computer/noq/commit/dd23094007e70592c12aa912f7b156f017ebaef1))
- Add comment to clarify need for custom iteration - ([4e6b8c6](https://github.com/n0-computer/noq/commit/4e6b8c6fe4fb1e056bd4ca7ea41fa240fbe31674))
- Refactor how ACKs are passed to the congestion controller - ([18ed973](https://github.com/n0-computer/noq/commit/18ed973568550ba044413d6a4a6cc8f51ff3fbbd))
- Inline single-use reject_0rtt() method - ([816e570](https://github.com/n0-computer/noq/commit/816e5701516db8a0e57931400924db9a3319227d))
- Handle handshake packets separately - ([6bddfde](https://github.com/n0-computer/noq/commit/6bddfdea1aaba04964ee902ce2f43200ca0c5e6d))
- Move PacketBuilder into a separate module - ([e1df56f](https://github.com/n0-computer/noq/commit/e1df56f40ced987ffbbc8a45f9706262415ea6b3))
- Move finish_packet() into PacketBuilder - ([7e0f3fa](https://github.com/n0-computer/noq/commit/7e0f3fa7ab6dbf4a42934787e544f7073719d347))
- Move more methods into PacketBuilder - ([d22800a](https://github.com/n0-computer/noq/commit/d22800ac79a20ccd1304a1dde4d15e2973b8a58f))
- Move probe queueing logic into PacketSpace - ([5a7a80e](https://github.com/n0-computer/noq/commit/5a7a80ec1eb4ec7c6d4a1f5677ac90d7da3d140e))
- Inline single-use congestion_blocked() method - ([1b92933](https://github.com/n0-computer/noq/commit/1b9293366f9cbef6e9164e89e6a016d310cf7642))
- Refactor handling of peer parameters - ([dde27bf](https://github.com/n0-computer/noq/commit/dde27bf9197b7291c6e0726807390a1576bb5359))
- Rename Streams to StreamsState - ([584b889](https://github.com/n0-computer/noq/commit/584b889494d48d70802866b62c2052f3faade4bc))
- Add public Streams interface - ([8bbe908](https://github.com/n0-computer/noq/commit/8bbe908dbddb4a0230a71159423713b5e9bc000d))
- Move API logic into Streams - ([d4bfc25](https://github.com/n0-computer/noq/commit/d4bfc25d6a88576d0d2585c63f0a18a9b67ee350))
- Split streams module into two parts - ([8131bcc](https://github.com/n0-computer/noq/commit/8131bcc7b6e1ac51eba00ced1199571a7c3797e8))
- Extract separate SendStream interface type - ([0b350e5](https://github.com/n0-computer/noq/commit/0b350e5d11541a03a7f5ad995ded4b967c443ac3))
- Extract separate RecvStream interface type - ([5a7b888](https://github.com/n0-computer/noq/commit/5a7b88893430a15167b35a83852ad7ef8312954c))
- Standardize on ch suffix - ([29e8a91](https://github.com/n0-computer/noq/commit/29e8a914d811a1ca3aa94a8cd58c92134b52c2fa))
- Inline single-use poll_unblocked() method - ([cc1218e](https://github.com/n0-computer/noq/commit/cc1218ed12ac1af4e45b275ec325b41f82c49cca))
- Inline single-use flow_blocked() method - ([a11abc6](https://github.com/n0-computer/noq/commit/a11abc648e7d67b1851592e5b29bff48f4647a2b))
- Inline single-use record_sent_max_data() method - ([afe8a6c](https://github.com/n0-computer/noq/commit/afe8a6cfeeff9dd7d63c04473e6d52446f05c14f))
- Move datagram types into separate module - ([71484b5](https://github.com/n0-computer/noq/commit/71484b57aaab68ddd63cb1b3af71f4a452585279))
- Derive Default for DatagramState - ([c6843a7](https://github.com/n0-computer/noq/commit/c6843a7c84992f9b3bc9bf28aef7d6d3b15cee2d))
- Move datagram receive logic into DatagramState - ([d25ce15](https://github.com/n0-computer/noq/commit/d25ce1523914df9215146d2dd8c3456af5216232))
- Move incoming datagram frame handling into DatagramState - ([2637dfe](https://github.com/n0-computer/noq/commit/2637dfe5758105a9ed9b4bfccee63913c62dd674))
- Move datagram write logic into DatagramState - ([61129c8](https://github.com/n0-computer/noq/commit/61129c811e319a43cd683230970c1876d72adfe6))
- Merge bytes_source module into connection::streams::send - ([4abf5a6](https://github.com/n0-computer/noq/commit/4abf5a64e021da6add842ad182d1d74417aa5ee5))
- Reorder code from bytes_source module - ([6292420](https://github.com/n0-computer/noq/commit/62924202fa0321def923054dcdbdbb77e241aabc))

## [0.6.1](https://github.com/n0-computer/noq/compare/0.6.0..0.6.1) - 2020-04-02

### H3

- Re-enable h3 async tests after busy-loop fix. - ([6f1d361](https://github.com/n0-computer/noq/commit/6f1d361dbf0c5d7818a26d9a3db29144f56030c4))
- Fix hidden warning - ([394cc8a](https://github.com/n0-computer/noq/commit/394cc8a7f2d5918ba8fe178b667ad54c6cc2b1bd))
- Make connection constructors infaillible - ([bf0dd08](https://github.com/n0-computer/noq/commit/bf0dd08024475755259b3e2a88d4e3f7cdaefb51))
- Request build helper macros for tests - ([b265648](https://github.com/n0-computer/noq/commit/b2656480786707468783032efdb0a10f84884cf5))
- Join timeout helper - ([02a4410](https://github.com/n0-computer/noq/commit/02a44108f4164d1c4e49dc4594abbb154294db0f))
- Reword comment - ([c1e4d86](https://github.com/n0-computer/noq/commit/c1e4d86d0daba1bcf7ab4077106b33221d0a30c0))
- Serve_one return its error instead of panic - ([67f7649](https://github.com/n0-computer/noq/commit/67f7649f82b4cc1b55d4c9616619c2d737723634))
- 0-RTT implementation - ([5e03cd9](https://github.com/n0-computer/noq/commit/5e03cd935f5b9b49b1ef75901cc9e2d34fd7ce94))
- Simplify complex destructurings - ([a244593](https://github.com/n0-computer/noq/commit/a2445933a656cbeb3c95e7322d392710e7b80b10))
- Activate qpack everywhere - ([14f5230](https://github.com/n0-computer/noq/commit/14f523019d66b12421a548dd3860575c8b50781c))
- End to end response cancellation impl - ([1be67f2](https://github.com/n0-computer/noq/commit/1be67f25971a0bb3ae0c774534c3b04049bced11))
- Rework Errors, easy to handle HttpError enum - ([4068d3f](https://github.com/n0-computer/noq/commit/4068d3fad20db0852b2c3a1695863f2a2c0e6305))
- End to end GoAway implementation - ([8bcca95](https://github.com/n0-computer/noq/commit/8bcca95304655e067ec5f57f76740f781dac17cf))

### Interop

- Parse size from full path - ([66e13e6](https://github.com/n0-computer/noq/commit/66e13e60b9df96a6feee18bebd018e6bb52e97b5))

### QPACK

- Ignore unknown stream cancellation - ([0e87485](https://github.com/n0-computer/noq/commit/0e874850e9b1e78093b4e13a7df761497bbe9296))

## [0.6.0](https://github.com/n0-computer/noq/compare/0.5.0..0.6.0) - 2020-03-12

### ⚙️ Miscellaneous Tasks

- Key update forcing test method - ([42c5cc8](https://github.com/n0-computer/noq/commit/42c5cc8a5473ba70fbbcc4ab079783b36fa5ac5e))
- Tracing for received frames - ([5f803f5](https://github.com/n0-computer/noq/commit/5f803f5615d6f1c129804f46e83196a59bb098b4))
- Tracing for Uni streams - ([1d82785](https://github.com/n0-computer/noq/commit/1d827856a358616a980b4c08edfe8b5d0db309db))

### H3

- Shorten client builder story - ([7e7262a](https://github.com/n0-computer/noq/commit/7e7262ab11982761b9e41c3b6df84e02da040195))
- Rewrite simple_client example - ([102a727](https://github.com/n0-computer/noq/commit/102a727629435b9f7397a44228593b1100847b45))
- First useful traces in client and connection - ([c8f86f8](https://github.com/n0-computer/noq/commit/c8f86f8fc00861a6c49aaceb9a041d59de578128))
- Remove unused local setttings from connection - ([88adcbe](https://github.com/n0-computer/noq/commit/88adcbe891cadcf2cf30e90e717b5e1d17751479))
- Refactor client to API into one BodyReader - ([4f5b196](https://github.com/n0-computer/noq/commit/4f5b196b5d74c9c4d2eacbc0a6001f8d2be1dce6))
- Refactor server to use only BodyReader/Writer - ([9fd975a](https://github.com/n0-computer/noq/commit/9fd975afa3358234016cacbc2f127340a170ca44))
- Rewrite introp client with the new API - ([e141526](https://github.com/n0-computer/noq/commit/e14152616392f250680889407bcc82f4ce0d83f5))
- Add async-only data stream interface for body reader - ([16344f8](https://github.com/n0-computer/noq/commit/16344f89bfea312a9d39bbf1628b8aa42c94998e))
- Keep only simple examples - ([c7504ad](https://github.com/n0-computer/noq/commit/c7504ad800b72dc6e6d9be1482474bbc80343e5b))
- Shorten server builder story - ([0b1c567](https://github.com/n0-computer/noq/commit/0b1c567fdd212d9c0ce1344da69567fcb7d48588))
- Remove priority frames - ([e19d9c6](https://github.com/n0-computer/noq/commit/e19d9c6bf16461638a2e1767716ce308cd2c9920))
- Forbid settings item duplication - ([bce5404](https://github.com/n0-computer/noq/commit/bce54043265adf9ff2cde0a9d71e7c21fcf9ca68))
- Forbid H2 reserved frame types - ([3876672](https://github.com/n0-computer/noq/commit/387667214801b7f57d7fa44f3302c1173b839102))
- Reserved SettingsId get the same pattern as frames - ([8e3e91f](https://github.com/n0-computer/noq/commit/8e3e91f6c66585c8f7ba3bf0d7697aa090a2eb27))
- Ignore reserved uni streams - ([3fc45b2](https://github.com/n0-computer/noq/commit/3fc45b2231065a3dc99164888f8cc1edfda0621b))
- Ignore reserved frames on concerned streams - ([bb63196](https://github.com/n0-computer/noq/commit/bb63196a4c97f0a7cf81f6099f4627646a9e15d3))
- Bump ALPN to h3-24 - ([79dc609](https://github.com/n0-computer/noq/commit/79dc609c0a61ea7385a9e71ac9947914ff7dec08))
- Allow connection with a custom quic client config - ([259b970](https://github.com/n0-computer/noq/commit/259b970bbc291b8a24b3d155a4d44f03f4c6585f))
- Temporary 0-RTT interface for interop - ([f56a884](https://github.com/n0-computer/noq/commit/f56a8843b440a23dcbba7ce7ddcee986054014ff))
- Default scheme to https - ([f21f01d](https://github.com/n0-computer/noq/commit/f21f01ded68834e6fd91ad1d2be3791e0d8d91c1))
- Consume reserved frames payload - ([03fbc79](https://github.com/n0-computer/noq/commit/03fbc79f0002bd234fa6df8179170c71f66a8003))
- Rename push_id field into id - ([2359715](https://github.com/n0-computer/noq/commit/23597158bd13cf90b1168643c2ff6dfc8056da62))
- Send a Set DynamicTable Size on encoding enable - ([86227f4](https://github.com/n0-computer/noq/commit/86227f4b4535614fffd30764615445566d2df367))
- Remove unlegitimate IOError in body reader - ([71d3969](https://github.com/n0-computer/noq/commit/71d39699d41f050278277ad36b9ce291a8012ae4))
- Poll control before anything in driver - ([cf1cac3](https://github.com/n0-computer/noq/commit/cf1cac35872912b21928c0ba8351a94c948ce84a))
- Avoid panics in server example critical path - ([f9965e4](https://github.com/n0-computer/noq/commit/f9965e473f750e0456f2346c0001048a7ac485f1))
- Accept directly http::Uri in client example args - ([21f9cc9](https://github.com/n0-computer/noq/commit/21f9cc9d334f2802a520268be7169c7bc45b20d2))
- Spawn each incoming connection in server example - ([0b4fa10](https://github.com/n0-computer/noq/commit/0b4fa10b970b06b2918b6b793079ddea0c264f71))
- Fix header len count in request constructor - ([6fc82a1](https://github.com/n0-computer/noq/commit/6fc82a1ffdb6bc3953355f5f99238b4bf7956665))
- Default path is "/", not "" - ([3befe15](https://github.com/n0-computer/noq/commit/3befe15c16f145626415426a732b1f6aad67387f))
- Poll control on opening instead of twice per loop - ([c07cc7f](https://github.com/n0-computer/noq/commit/c07cc7f9d22dc9a66a049fb9b8ba879508436bd8))
- Fix WriteFrame incomplete write management - ([7699d33](https://github.com/n0-computer/noq/commit/7699d33c4e221251e9f00234ef6956c0ffe4c7e8))
- Do not copy sent payload - ([4a8cd6a](https://github.com/n0-computer/noq/commit/4a8cd6af3595ef944d5121e26c30887311088ba8))
- Display frame type number when it is unsupported - ([1a8d825](https://github.com/n0-computer/noq/commit/1a8d825ab8c65c937d060aab41e5f38154a7dd1c))
- Make frame decoding resilient to split headers - ([384549f](https://github.com/n0-computer/noq/commit/384549faf218a76b3dd7b9c712f6a192229b7522))
- Close body writer on any state - ([ff4d736](https://github.com/n0-computer/noq/commit/ff4d736c06ab613bf8aee522d32d946bf805a862))
- Use early return where possible. - ([02fb796](https://github.com/n0-computer/noq/commit/02fb7969088299c2784964b65417bc05046d9ca4))
- Fix driver error reaction for closing and logging - ([d4b0553](https://github.com/n0-computer/noq/commit/d4b05530adf1b519143fc00a9848293b3c3d0634))
- Make Request yeild Connection's private - ([949d947](https://github.com/n0-computer/noq/commit/949d9473942ff4d4d127cede75958df1713b4e37))
- Make user's futures resolve on driver error - ([445a438](https://github.com/n0-computer/noq/commit/445a4388aa1753bff363de6b741c94b25018ef21))
- Update tokio dependency to 0.2.6 - ([b2fac76](https://github.com/n0-computer/noq/commit/b2fac762f0d124587ea3f37f3d853ef6966296ed))
- Close connection on client drop - ([757deec](https://github.com/n0-computer/noq/commit/757deecbdf8d30aa1e582fe30493f4f52e64bf09))
- Functionnal test for connection closure - ([85a012e](https://github.com/n0-computer/noq/commit/85a012e9df9731d2556db753f3ab6351f1b9007a))
- Remove superfluous parenthesis - ([d1127d5](https://github.com/n0-computer/noq/commit/d1127d5c643b09b916936f2b320801e45e96365a))
- Throughput benchmark - ([4257846](https://github.com/n0-computer/noq/commit/42578464fea2369c0e3e0894b5d67fc90542c29e))
- Save an allocation on frame header encoding - ([e54042a](https://github.com/n0-computer/noq/commit/e54042a038db357f48be993f4dcd26ff7e3e3838))
- Test request body reception - ([cb9597b](https://github.com/n0-computer/noq/commit/cb9597b3bfabbd779bc1bc9172bc060df8738174))
- Minor style fix - ([3613974](https://github.com/n0-computer/noq/commit/3613974b94189b6e8de7c33061db0c69c4aec1a9))
- Remove commented code left by error... - ([705269b](https://github.com/n0-computer/noq/commit/705269b6c0a03a0abbbeed2ba9fc88dda8876ec8))
- Remove NUM_PLACEHOLDERS settings - ([efee187](https://github.com/n0-computer/noq/commit/efee1871db62060e7a8187ceb3ff797e6a7d8a1e))
- Refactor settings in it's own module - ([69ee3e5](https://github.com/n0-computer/noq/commit/69ee3e5f57a380b90427796f6516775f8c85d27c))
- New settings for interop - ([723fc97](https://github.com/n0-computer/noq/commit/723fc97e77ebf1a241bffa3d090c11f53a060d19))
- Inline crate's default settings - ([a2343c8](https://github.com/n0-computer/noq/commit/a2343c8df7a38712a5ac0d690532df4e8dbd5db0))
- Don't run request before client closure tests - ([75aa21b](https://github.com/n0-computer/noq/commit/75aa21b446221e0ec689a9fb679f64c488afbe6f))
- Set FrameDecoder initial buffersize to UDP size - ([0ff3d8f](https://github.com/n0-computer/noq/commit/0ff3d8f51db767e7ac6b963ead07ae25c384882a))
- Disable async tests in coverage - ([6c557ff](https://github.com/n0-computer/noq/commit/6c557ff5efb6f3a5696d849754967b6ac2d60107))

### Interop

- Do not check h3 on hq only endpoints - ([43bbeaa](https://github.com/n0-computer/noq/commit/43bbeaadd19fb3ee996180a309eefbb5d34ad3e0))

### QPACK

- Track two ref blocks per stream - ([3b4b86a](https://github.com/n0-computer/noq/commit/3b4b86a9f814e0ce7541d529f7f722a52da384b0))
- Make dynamic tracking state non-optional - ([fc1035f](https://github.com/n0-computer/noq/commit/fc1035ff3c0690e49ecad415641e711215de4c9f))

### QUINN

- Include ios in the conditional compilation for mac platforms - ([605c9a5](https://github.com/n0-computer/noq/commit/605c9a57efd89055118232fbb9eee3728e68ffbb))
- Allow retrieving the peer's certificate chain from a connection - ([7122eab](https://github.com/n0-computer/noq/commit/7122eab85712b15b598998b324f3e777bed57ae6))

## [0.5.0](https://github.com/n0-computer/noq/compare/0.4.0..0.5.0) - 2019-12-03

### ⚙️ Miscellaneous Tasks

- Throw an error when client recieves a BiStream - ([46102d0](https://github.com/n0-computer/noq/commit/46102d07f2986dd349792308a74d3786de0f6aef))

### H3

- Sending headers gets it's own future - ([df4880a](https://github.com/n0-computer/noq/commit/df4880a93602d127939f4a4523571ebe21afac33))
- BodyWriter, AsyncWrite implementation - ([9b212aa](https://github.com/n0-computer/noq/commit/9b212aaef547571b71a56bbf5a60069c7c57fa2e))
- Refactor server code to integrate BodyWriter - ([fdd801c](https://github.com/n0-computer/noq/commit/fdd801c6e96de13b4c8449b9aa95148047f71163))
- Fix tail buffer ignored in BodyReader - ([0459854](https://github.com/n0-computer/noq/commit/0459854acbc7663aada5cf0eae9dae932aded773))
- Use SendHeaders to send trailers in SendResponse - ([2e7ef52](https://github.com/n0-computer/noq/commit/2e7ef52d0d2c12b4311c5d6f90533fceee932bc3))
- Refactor client with SendHeaders - ([6d6763d](https://github.com/n0-computer/noq/commit/6d6763da030aa004754782bf24e7674b658a6991))
- Make sending response error management more ergonomic - ([436f1cd](https://github.com/n0-computer/noq/commit/436f1cd511bf831ca52a187fa793daee322c560f))
- Introduce builder pattern for client request - ([6950e35](https://github.com/n0-computer/noq/commit/6950e350eac26a9da8d3898bfb6ea3946583bbbd))
- Helper function to build response - ([07cc1cc](https://github.com/n0-computer/noq/commit/07cc1cc5aee83fb66e8cfd73c9fc20cd4e2d7589))
- Stream request body from client - ([d6b696c](https://github.com/n0-computer/noq/commit/d6b696c192d5dfe0601b9d5acfa6e8765910c2f6))
- Prevent extra copy when sending DataFrame - ([3bd0c69](https://github.com/n0-computer/noq/commit/3bd0c6969deff1f17c54361d18b4731f42720643))
- Rename Response and Request Builders - ([accb344](https://github.com/n0-computer/noq/commit/accb344fe0c6c675b46d5753a036288e14a67b3d))
- Let client close connection gracefully - ([36dfee6](https://github.com/n0-computer/noq/commit/36dfee6ea7deed70950e8e1b14f36595d237fe9c))
- Minor readabilty tweak - ([297c99a](https://github.com/n0-computer/noq/commit/297c99a9aa571af0b7ca9da5d7d4680912ed0a5a))
- Move some common example code into a shared module - ([69a4977](https://github.com/n0-computer/noq/commit/69a49772675837b0540545fdac9b981a29508370))
- Simpler examples - ([5881196](https://github.com/n0-computer/noq/commit/588119621193826d3f5eb725b5f57d921dee16c9))
- Incoming UniStream header parsing and polling - ([248ec17](https://github.com/n0-computer/noq/commit/248ec17f0316b3771d1d7a2f0f5be241b71c2810))
- Poll incoming uni streams from connection - ([18a9532](https://github.com/n0-computer/noq/commit/18a95328b94f674a22cb9e10517c211c809189af))
- Do not poll incoming bi streams in client - ([a4e6563](https://github.com/n0-computer/noq/commit/a4e656302b3f9f125805a461de3f66d8f2c16298))
- Make Settings and SettingsFrame the same type - ([ca02516](https://github.com/n0-computer/noq/commit/ca0251640afdc6908cbf0ca02186c2235fab9a38))
- Control stream implementation (Settings only) - ([90f6ce1](https://github.com/n0-computer/noq/commit/90f6ce10d132def4d320ea22d8d757f9ad94b24f))
- Control stream sending mechanism - ([ad4f516](https://github.com/n0-computer/noq/commit/ad4f516f0f9e0463ed3e3b47ac263e0fa240358a))
- Filter control frame types for client or server - ([2bb13fa](https://github.com/n0-computer/noq/commit/2bb13fa6c5c8be79844c956f0e7d62f944b8baf2))
- Immediately close quic connection on fatal errors - ([0bec6ea](https://github.com/n0-computer/noq/commit/0bec6eac8eb6458c950d9153ff40c7587b454197))
- Track ongoing requests - ([a570f17](https://github.com/n0-computer/noq/commit/a570f17daab80c61ca6763aceb6d3241ff811cc5))
- GO_AWAY implementation - ([f170a89](https://github.com/n0-computer/noq/commit/f170a8992266abc6aae07b3aaa8fb268606e6175))
- Rename RecvRequestState finished variant - ([0972700](https://github.com/n0-computer/noq/commit/0972700dab8681146abd3b6e43de4c8d1c20bd9e))
- Typo in ResponseBuilder name - ([51e5aae](https://github.com/n0-computer/noq/commit/51e5aaebbce864a06e100e04a952658a1d78f41d))
- Issue quic stream errors and reset streams - ([d4caaf5](https://github.com/n0-computer/noq/commit/d4caaf5cd4a6b7b4eee5633ac3244ec0ca0410a1))
- Rename ReadToEnd's State - ([ec67124](https://github.com/n0-computer/noq/commit/ec671244828b2f82a45263ba55579e971dac639f))
- Request cancellation and rejection - ([c2cbffc](https://github.com/n0-computer/noq/commit/c2cbffc4830de6c814c75f7cc8fc28bdd31fd1b2))
- Better error reason when control stream closed - ([efde863](https://github.com/n0-computer/noq/commit/efde8638c25981c8f3296dc036df0fd1e108d12a))
- Move ErrorCode to proto - ([cf2c46a](https://github.com/n0-computer/noq/commit/cf2c46abd9ccb46b42ab494f2c2b0540ad122121))
- Fix driver polling story trivially - ([a8eb51b](https://github.com/n0-computer/noq/commit/a8eb51b01c30275c91f3fd5ddd2cf5adddcb509b))
- Fix freshly type-resolved incoming uni streams handling - ([06db4fb](https://github.com/n0-computer/noq/commit/06db4fb7571cce6168dd0b2e7f0080180db92e37))
- Replace SendControlStream with a generic impl - ([a87e1a9](https://github.com/n0-computer/noq/commit/a87e1a9cf7c3904f6878c0e922ea71e1d9259bbb))
- Lock ConnectionInner once per drive - ([8b40524](https://github.com/n0-computer/noq/commit/8b405240b75c0c61a01213d11ae7e07aa77c32a5))
- Manage all Uni stream transmits the same way - ([daf9dc3](https://github.com/n0-computer/noq/commit/daf9dc3153d3f20ccea1c1050584da01c4ec76f7))
- Move Connection::default to tests - ([fe6935d](https://github.com/n0-computer/noq/commit/fe6935d3f25203af51d16d8c21714b7bd07a8725))
- Resolve encoder and decoder streams - ([4921821](https://github.com/n0-computer/noq/commit/4921821a7d4e30dc1b66f4e82d068bee51c55f41))
- Set encoder settings on receive - ([77d32c3](https://github.com/n0-computer/noq/commit/77d32c3352ccecbf281122d3edea36080cb7d71a))
- Pass required ref to connection's decoding - ([4572499](https://github.com/n0-computer/noq/commit/45724990484e43d4b9b32a9d0becb7e3b42330ed))
- Unblock streams on encoder receive mechanism - ([e07b70e](https://github.com/n0-computer/noq/commit/e07b70ea9161fa5caf739def25d9882b1ca16def))
- Receive decoder stream - ([5e6b83b](https://github.com/n0-computer/noq/commit/5e6b83b0c4fc36a507cca1fbb4efcaecbeae26df))
- Send decoder stream after decoding a block - ([81f1fd2](https://github.com/n0-computer/noq/commit/81f1fd2a45565e7d8f938871dd893a664516f4f1))
- Do not ack headers not containing encode refs - ([a8e9394](https://github.com/n0-computer/noq/commit/a8e93949aa3acb6c5462042b19208489c2b47da9))
- Fix and optimize new StreamType decoding - ([dff7eee](https://github.com/n0-computer/noq/commit/dff7eeed200204fef91d4a3edb84ddefa39d4553))
- Enable QPACK by default - ([96011ca](https://github.com/n0-computer/noq/commit/96011ca890bc8d7fb4f40a76f36da058a36a4fb5))
- Move connection constants to the bottom - ([71b72eb](https://github.com/n0-computer/noq/commit/71b72eb1a94484687a01d24ecdaff85c0286b6ac))
- Add QPACK error codes - ([d184b18](https://github.com/n0-computer/noq/commit/d184b18845e083c71fc264f47c6b3c4bde5fcde5))
- Move actual drive impl to ConnectionInner - ([8c85c41](https://github.com/n0-computer/noq/commit/8c85c411a29d9f6dfb246d08e916dc6b756b2b96))
- Let internal error messages be strings - ([418b0a7](https://github.com/n0-computer/noq/commit/418b0a7f638489ce1712d3bda29f976169155c66))
- DriverError to carry connection level error - ([a36ffb8](https://github.com/n0-computer/noq/commit/a36ffb824db507146ce4ee100fbd6a5562de0dfc))
- Replace all driver error repetitive impls - ([24a10ba](https://github.com/n0-computer/noq/commit/24a10baecd1f18277a996e7737d00b4fab38356b))
- Set quic connection error from top driver level - ([a61c960](https://github.com/n0-computer/noq/commit/a61c96058677fb680a70371e5b6ce2ff896c890c))
- Fix formatting - ([bf4c5a2](https://github.com/n0-computer/noq/commit/bf4c5a253040fe629074d7c3a518319778f37792))
- Better recv uni stream buffer space management - ([84e0419](https://github.com/n0-computer/noq/commit/84e041939d94e620a7b9c79af2796d8574596e6d))
- Simplify SendUni state machine - ([69a9545](https://github.com/n0-computer/noq/commit/69a95456326d97feb5bfe363caa0edc2ec168b5c))

### QPACK

- Prevent substraction underflow in VAS - ([5b67ded](https://github.com/n0-computer/noq/commit/5b67dedf842da79265e48a19d25b583e7d632f59))
- Rename mem_limit to max_size, as in specs - ([807ae06](https://github.com/n0-computer/noq/commit/807ae068cf40cfb454fcf448bf65f4e31dad7366))
- Do increment largest known received ref - ([aebada3](https://github.com/n0-computer/noq/commit/aebada353be7949a44331215e3a76841f74548cb))

### H3

- Std futures - ([f5e014d](https://github.com/n0-computer/noq/commit/f5e014dae1f6b1dcb240e991aefa1a0e8682477c))

## [0.4.0](https://github.com/n0-computer/noq/compare/0.3.0..0.4.0) - 2019-09-08

### ⚙️ Miscellaneous Tasks

- AsyncRead implementation for recieving body from client - ([db7a8d3](https://github.com/n0-computer/noq/commit/db7a8d3dd3fd6f38b9754000ae9edf49d4bf5248))
- Default capacity values for RecvBody - ([57c756a](https://github.com/n0-computer/noq/commit/57c756a633d90ea62cef5d3d795845768c54a06e))
- Make the user specify memory usage params on RecvBody construction - ([a90ad6e](https://github.com/n0-computer/noq/commit/a90ad6e8b87e15a4d70676bfea107f3fdbd958c8))

### ClientConfigBuilder

- :logger - ([3298fc9](https://github.com/n0-computer/noq/commit/3298fc91bc36467b4699e0617199d1668a6b1c70))

### H3

- Add QPACK Settings in h3::frame::Settings - ([2053e56](https://github.com/n0-computer/noq/commit/2053e564f46932d3adb7aeba96636d2d382071e4))
- Move codecs to a new proto module - ([eab98c1](https://github.com/n0-computer/noq/commit/eab98c19f8bc4ce60bcadcf834ecddb0efc81ffb))
- Future::Stream for HttpFrames - ([82bd19c](https://github.com/n0-computer/noq/commit/82bd19c8bb08a1cac6ec8f0482e1fae68c27c121))
- Builders for client and server - ([18cef27](https://github.com/n0-computer/noq/commit/18cef2753614b3f3aab759b0b7d99a1740a13596))
- Connection types, common to server and client - ([af7883a](https://github.com/n0-computer/noq/commit/af7883a99d57b34bb393c60843ed776df3c08280))
- Server incoming connection stream - ([1740538](https://github.com/n0-computer/noq/commit/1740538157320fb59b0cdc32d40076ab2c289e80))
- Connecting wrapper for client - ([5658895](https://github.com/n0-computer/noq/commit/565889591da797d3b14d25faaf5a3fe91c7cf044))
- Introduce client+server example, with connection story - ([452cdd5](https://github.com/n0-computer/noq/commit/452cdd532ad148ae4a72f4bdd2f27d6879c90380))
- Let encoder pass an iterator instead of a slice - ([79d07bd](https://github.com/n0-computer/noq/commit/79d07bd67fa87d8e40b995b5c9a9abece90bebc1))
- Make max_header_list_size unlimited by default - ([bd8cc90](https://github.com/n0-computer/noq/commit/bd8cc901e6fb5c7551ee960582a45930f0af1983))
- Encode headers from inner connection - ([0bb05fc](https://github.com/n0-computer/noq/commit/0bb05fc93b606ce07163fc3ce86d49fd2e576eca))
- Set qpack params when constructing connection - ([72bb118](https://github.com/n0-computer/noq/commit/72bb118c274f357043424bcbc918853d90779703))
- Header decoding - ([9c484ec](https://github.com/n0-computer/noq/commit/9c484ecef5480f7c9ac14fef581618804a91d1ea))
- Make stream id value accessible from SendStream - ([cbd22d6](https://github.com/n0-computer/noq/commit/cbd22d6b06b8bcc18ab824a748592cc0aa7e9908))
- Basic send request future for client - ([d1d0915](https://github.com/n0-computer/noq/commit/d1d0915afdc2c1fd25f4761299045c7b1520a061))
- Receive request for server - ([9c5a777](https://github.com/n0-computer/noq/commit/9c5a777bf2cd8618dca2e0594b45fb5f33f55946))
- Incoming request stream - ([92a3f20](https://github.com/n0-computer/noq/commit/92a3f20842c0ca676de018c8965c362d68640eff))
- Pseudo header handling for `http` crate integration - ([af8ba54](https://github.com/n0-computer/noq/commit/af8ba54dd5d64f9e73fe9804c07ed2d8f1a6e005))
- Integrate Header type for encoding / decoding data types - ([2ece5f9](https://github.com/n0-computer/noq/commit/2ece5f9939909de01d74e561403d3cf21d3fc3b6))
- Make example send / receive request in client / server - ([8bf597d](https://github.com/n0-computer/noq/commit/8bf597db68b3bcad2f0cf23f4d5ec2002cfdaba1))
- Make server receive a Request struct - ([3b79240](https://github.com/n0-computer/noq/commit/3b79240e447432444098ca22643cd9ed01aeb2de))
- Send Response from the server - ([82cb3ce](https://github.com/n0-computer/noq/commit/82cb3ce285bc19ccdcd73260f48b87a7a6df0545))
- Make client receive a Response struct - ([da2edba](https://github.com/n0-computer/noq/commit/da2edba29124df2fb3e0d04233acb2d33dadb480))
- Generalize try_take helper usage - ([051ab91](https://github.com/n0-computer/noq/commit/051ab91df9eb88d8ae05b90ad148a5302d30db91))
- Send body from server - ([af60668](https://github.com/n0-computer/noq/commit/af606683a03f74daed400f2d91cf51498b3bf03c))
- Fix infinit FrameStream polling (don't ignore poll_read() = 0) - ([fca903d](https://github.com/n0-computer/noq/commit/fca903d1b59c8517e8c08a341d254141fb6d5fc5))
- Client receive body - ([5504ffa](https://github.com/n0-computer/noq/commit/5504ffafa83bd5199442bb5c5c7929357a7b881c))
- Exchange trailers after body - ([2e8a2fb](https://github.com/n0-computer/noq/commit/2e8a2fb1e4e070c86515287f8e7683d9a8c07d4f))
- Fix frame stream not polled anymore when finished - ([9fcf929](https://github.com/n0-computer/noq/commit/9fcf9290ba1754d9e6a33b3a5363f8b19cec694e))
- Request body - ([efaf945](https://github.com/n0-computer/noq/commit/efaf945258bb318be1dbeec62531247e1d9a0ecd))
- Send trailers from client - ([25fc68d](https://github.com/n0-computer/noq/commit/25fc68d760048e12054e69f90c6f568c4785124a))
- Fix receive misspelling - ([e0f1d11](https://github.com/n0-computer/noq/commit/e0f1d11fb447e8530456b880c0478cef4e9706a4))
- Document pseudo-header fields - ([4c75c06](https://github.com/n0-computer/noq/commit/4c75c06869ac8cb166d6d4ab7ecb7fcd5a759de8))
- Stream response from client - ([07eca3d](https://github.com/n0-computer/noq/commit/07eca3d34887ba05cba0ece21dd6c3f34b285307))
- Code reformatting from fmt update - ([e2ee96d](https://github.com/n0-computer/noq/commit/e2ee96de60232b446cdae68d54f7f053554fa2c7))
- Reset expected frame size once one have been successfully decoded - ([ded85aa](https://github.com/n0-computer/noq/commit/ded85aa004c5323552411c96fa0317eb76b2a44d))
- Use AsyncRead into the example - ([06d060c](https://github.com/n0-computer/noq/commit/06d060c0e911747478ba387b431a5296d08895ca))
- Separate request header, body, and response structs in server - ([92c04c3](https://github.com/n0-computer/noq/commit/92c04c3b66cbc25e9d130284c6c203157af512bc))
- AsyncRead or Stream from RecvBody, so server can stream request - ([874dafe](https://github.com/n0-computer/noq/commit/874dafefc7d2fb7a7074c2aa51cc4ed8def0d300))
- Return RecvBody along response in client, similarly to server - ([13cd3cf](https://github.com/n0-computer/noq/commit/13cd3cf86fa487b687aba68ff32cc3a6d72e696e))
- Introduce an intermediary type before any body-recv option - ([73be859](https://github.com/n0-computer/noq/commit/73be859e8c15ce441d71241fddf7d7ebd2dcd08e))
- Rename RecvBody into ReadToEnd and Receiver into RecvBody - ([73065fa](https://github.com/n0-computer/noq/commit/73065fa443181fbbd8d7147ae8de95b8f5587b37))
- Implement Debug for RecvBody - ([81aa76b](https://github.com/n0-computer/noq/commit/81aa76bc50129a53c609da4c63e9bc8360cae087))
- Embed RecvBody into Http:: Request and Response type param - ([503de0b](https://github.com/n0-computer/noq/commit/503de0b78e79112220b6250971b0d27b48c384b9))
- Remove superfluous stream / reader conversion for ReadToEnd - ([3408899](https://github.com/n0-computer/noq/commit/34088990dba6423aba4d2e52338e16c445a56bfc))
- Use ok_or_else to handle request headers building error - ([58bdb79](https://github.com/n0-computer/noq/commit/58bdb79497981dbc4ba2def25aeb3b5a0c0ac28c))
- Fix request / response build error handling - ([6c5dabe](https://github.com/n0-computer/noq/commit/6c5dabebe99eac23edf9fac5712246304b134736))
- Fix minor style problem - ([6ae1fb2](https://github.com/n0-computer/noq/commit/6ae1fb2504b23d377fbc6e16643cef48f0aa386a))
- Partial DataFrame decoding implementation - ([23dcf2c](https://github.com/n0-computer/noq/commit/23dcf2cf9077cd7e20b19410878051a35fce74c5))

### QPACK

- Make encode accept slice of HeaderField - ([9910013](https://github.com/n0-computer/noq/commit/9910013a4e30d1d4cbff6dd4279ec5fb8a8197f1))

## [0.3.0](https://github.com/n0-computer/noq/compare/0.2.0..0.3.0) - 2019-04-18

### Connection

- :close by reference - ([818dadd](https://github.com/n0-computer/noq/commit/818dadd671f049f40c6e25452456a42c71690d29))

### Endpoint

- :close helper to close all connections - ([ad6f15a](https://github.com/n0-computer/noq/commit/ad6f15a2660bb3f43df4e2ebd912f96c637bf8ef))

### H3

- Correct the placehoder setting type code - ([8389489](https://github.com/n0-computer/noq/commit/83894896fba1d04ce5a7fdbfe4ac968d3cf734d6))
- Fix setting ids in tests - ([55c5ae2](https://github.com/n0-computer/noq/commit/55c5ae298bd37549a04ff2ab2369ce970de37052))
- StreamType for unidirectional streams - ([f939f8e](https://github.com/n0-computer/noq/commit/f939f8ebe4d01f60a90966c40bd21e119bd9c560))
- Frame header reordering and varint for frame type - ([6a942a3](https://github.com/n0-computer/noq/commit/6a942a3c8eb438f7520645ce00ccfc7db6c95dae))
- Stream types varint format - ([54aa9c9](https://github.com/n0-computer/noq/commit/54aa9c967d26cb8a193cd5e9af061da4f4c3ed09))
- Varint Settings ids and ignore unknown settings - ([c2db1d5](https://github.com/n0-computer/noq/commit/c2db1d5437c250634e3e32d55ad11354badce4b9))
- Change reserved stream type pattern - ([af8ff7c](https://github.com/n0-computer/noq/commit/af8ff7c49e24c59dc3ac8fdd73b856e4db986a6d))

### QIF

- Get path from cli args - ([d9bc7ce](https://github.com/n0-computer/noq/commit/d9bc7ce8de0ca8c832512afc12189758ddc8d67a))
- Correctly set max table size - ([a2dea7c](https://github.com/n0-computer/noq/commit/a2dea7c6401a3955ffd5db6e350d1950256baa77))
- Encode one file, without configuration - ([2997382](https://github.com/n0-computer/noq/commit/2997382aa9cd7cad95ba14bb05e8f1ba6a9d4915))
- Iterate over qif sir and generate all encode cases - ([06654e2](https://github.com/n0-computer/noq/commit/06654e2e7f9893d61a3e24b8a89e6c92ebea864b))
- Implement acknowledgement mode - ([a1cc9ca](https://github.com/n0-computer/noq/commit/a1cc9caebb86464dad7907a642bd8363602ae1df))
- Handle encoded files for all impls, generalize failure display - ([b762712](https://github.com/n0-computer/noq/commit/b76271280b039485b3e749e63eb88290d61f1318))
- Gather encoding results - ([b710fcc](https://github.com/n0-computer/noq/commit/b710fcc1a50ad27b08e1f50a63edf43cf6b85149))
- Get encoder settings from cli args - ([0c37a8a](https://github.com/n0-computer/noq/commit/0c37a8a9943904fcf1af574b4d4d3558525e381f))
- Use cli args when encoding a single file - ([9dc05fe](https://github.com/n0-computer/noq/commit/9dc05fe5d8b0a9a44380df12eb4115eba9b4b71f))
- Handle blocked streams - ([15b3cbb](https://github.com/n0-computer/noq/commit/15b3cbb346749b1c89e3a470301532107a992a06))
- Use max blocked stream in encoding and check validity on decoding - ([f874a8d](https://github.com/n0-computer/noq/commit/f874a8dad6d10b82e697946daf23d2f64618f6d3))

### QPACK

- Retreive fields by name in static table - ([ceed37e](https://github.com/n0-computer/noq/commit/ceed37e2edd18a98c17ad917a4c92d7b04b34590))
- Reformat after big rebase - ([664ecef](https://github.com/n0-computer/noq/commit/664ecef22de1b2aa0f132c08a50cff65b6ae8f7a))
- Rewrite prefixed integers using Codec traits - ([76cd18b](https://github.com/n0-computer/noq/commit/76cd18b7c86a110c8a9641bb4234ac02f75a7d90))
- Rewrite prefixed string using codec traits - ([ed37636](https://github.com/n0-computer/noq/commit/ed3763680588771cfb59b59403cc9830820cf72e))
- Rework decoder to use prefix_* mods and remove unused code - ([772d873](https://github.com/n0-computer/noq/commit/772d8739bb980747bc304972a72388a909c3495b))
- Get largest reference from VirtualAddressSpace - ([539b35f](https://github.com/n0-computer/noq/commit/539b35f2871f3c2de5a36a65226f2a7af2275bcd))
- Fix last post base index exlusion - ([b509260](https://github.com/n0-computer/noq/commit/b50926027f00be501d11c3693ea8329edf1b1782))
- Header bloc decoding implementation - ([38f925f](https://github.com/n0-computer/noq/commit/38f925f99b6aec57780d814de8073e7a119ce403))
- Simplify name reference code with header field value method - ([40899b2](https://github.com/n0-computer/noq/commit/40899b2e355ff265be6b8e857a5f0c97b018bb9c))
- Refactor error decoder handling - ([1528ac6](https://github.com/n0-computer/noq/commit/1528ac60cbd24fa866cef0b6e0485cea655d1554))
- Refactor encoder stream decode function - ([12b1b25](https://github.com/n0-computer/noq/commit/12b1b25892e61936393cc1d2591cba3a3afc6491))
- Add test when entries dropped, and base index calculation, fix vas - ([4f1d6b9](https://github.com/n0-computer/noq/commit/4f1d6b95f8353c6a9650f04788c439ef4550227e))
- Refactor decoder tests - ([e16ef57](https://github.com/n0-computer/noq/commit/e16ef57f7a32c6f640abf7f8f9387597609a6d00))
- Send Table state synchronize message back to the encoder - ([e4d490f](https://github.com/n0-computer/noq/commit/e4d490f843ba3ecfb1ba082e32f57e5bdbcf875b))
- Fix incomplete message parsing consuming too much bytes and breaking - ([d86bc92](https://github.com/n0-computer/noq/commit/d86bc928ce5114ae8859666839b6f684627087d1))
- Refactor encoder stream instruction outside decoder - ([aa0ba29](https://github.com/n0-computer/noq/commit/aa0ba2970ab578e00de0d23af05d29e6a91599b4))
- Refactor header bloc codec into it's own module - ([4f84488](https://github.com/n0-computer/noq/commit/4f84488d8ec2335cb948e3eed0f073d601550909))
- Use base index only when it is meaningful - ([f277603](https://github.com/n0-computer/noq/commit/f2776030d2ab822624d9cd3c46daeae217c731f0))
- Split stream inserter / bloc decoder interface: - ([79de3ca](https://github.com/n0-computer/noq/commit/79de3ca989c45d001b99495579fdc73f2ef42e5f))
- Retreive static index from name or name+value - ([ca92f2f](https://github.com/n0-computer/noq/commit/ca92f2fa00bc8888dfc9e3bf3740fb09cfb7fa5e))
- DynamicTable for the encoder - ([9195df0](https://github.com/n0-computer/noq/commit/9195df075975c81e9fbb938e91287ab43934ed61))
- Use tuple struct syntax for Duplicate encoder stream instruction - ([2a1e958](https://github.com/n0-computer/noq/commit/2a1e9588a2bc9ac3275bb640ac385a6fa5ebcb70))
- Static name reference insertion in dynamic table - ([4adccc4](https://github.com/n0-computer/noq/commit/4adccc4163cd5dd8dd2cc599912c2bce03527032))
- Known the value of an invalid prefix, fix Literal prefix check - ([12c0dab](https://github.com/n0-computer/noq/commit/12c0dab28fe71eaaa5ec8e5c61db59f4fc2d7187))
- Header bloc prefix codec - ([dceca45](https://github.com/n0-computer/noq/commit/dceca45476396e72b593fb9f967a86baccf93324))
- Encoder implemetation, without reference trancking - ([e2e07de](https://github.com/n0-computer/noq/commit/e2e07ded5cd76a78453e2579a6ed0de00c42037e))
- Retreive abolute index from real index - ([67ae321](https://github.com/n0-computer/noq/commit/67ae321eaccfc0f84b0f538ef6dc32116fe390b5))
- Reference tracking on encoding - ([d76cf81](https://github.com/n0-computer/noq/commit/d76cf81de208210154ea4d532dd1534ac343a834))
- Decoder instructions - ([bdf1ffc](https://github.com/n0-computer/noq/commit/bdf1ffc6ee7cff7269dc4e563ae37874feddef55))
- Use tuple structs for decoder stream types - ([2e1c3a0](https://github.com/n0-computer/noq/commit/2e1c3a0d4973628bff992a88ed637a7f6b7ad219))
- Untrack a bloc with stream id - ([70a0e0e](https://github.com/n0-computer/noq/commit/70a0e0e1b79c92e9c4299bc94138e23a0589bfae))
- Decoder stream impl - ([b47b18f](https://github.com/n0-computer/noq/commit/b47b18fd9ce182230dfa62fc94f31d51d8e31ebc))
- Update quinn-proto version - ([71602a9](https://github.com/n0-computer/noq/commit/71602a980e92035873030c016febb647e4268555))
- Test instruction count incrememnt - ([9b0308b](https://github.com/n0-computer/noq/commit/9b0308ba5b1ea8e82a1aa917c4ef4ae13ea249a8))
- Update name / name_value index maps on insert - ([3f00c49](https://github.com/n0-computer/noq/commit/3f00c49b057878c15a8cbf39d3f8aa70deec756e))
- Do not panic on tracking an already tracked stream bloc - ([a6d74bf](https://github.com/n0-computer/noq/commit/a6d74bfcb8a959b60e2e92c6772cd3d9b69c136f))
- Max table size = 0, fix division... - ([1c921ad](https://github.com/n0-computer/noq/commit/1c921addd72dc999b1c4ab99bed4595493c8266e))
- Tuple struct for TableSizeUpdate - ([76a33e2](https://github.com/n0-computer/noq/commit/76a33e2247f2300baf2590c1fc1b94f4e8151532))
- Codec tests - ([3f8de36](https://github.com/n0-computer/noq/commit/3f8de36512b456faa513774bd1dedb4339426212))
- Remove dead_code attributes - ([ab8a2c2](https://github.com/n0-computer/noq/commit/ab8a2c210208fa44e96ce141c576ce43a5647c58))
- Visibility cleanup - ([8d14573](https://github.com/n0-computer/noq/commit/8d145736f70716f5fe77e5b866100d97986b2bb1))
- Last public API impl - ([1bdf8d8](https://github.com/n0-computer/noq/commit/1bdf8d8acdb500f6aad11e2ad35d0583ee7efd54))
- Rename `bloc` to `block` - ([53f5934](https://github.com/n0-computer/noq/commit/53f5934bf632f9247e1788da692ccd2f1f3ae61a))
- Display header field in qif line format - ([0e15861](https://github.com/n0-computer/noq/commit/0e15861638930282575fdf02a498aa2e97be2c8a))
- Offline interop without encoder stream support - ([3564549](https://github.com/n0-computer/noq/commit/35645490c50c8bcc10decbd0dd92a0552cfcf385))
- Interop tool, encoder stream and failure summary - ([e0e9c50](https://github.com/n0-computer/noq/commit/e0e9c50375c71e9c4a6c9bb10cf348b2714fec64))
- Qif compare and better display - ([db02128](https://github.com/n0-computer/noq/commit/db02128974f8755614a5784cc8f58f6a8dd83b2c))
- Fix error when required_ref = 0 and delta_base = -0 - ([5254d2f](https://github.com/n0-computer/noq/commit/5254d2ffd900a07a6be3e1387b8571dd5be0efbc))
- Tracked blocked streams, do not insert if max reached - ([7fb4356](https://github.com/n0-computer/noq/commit/7fb435641029cbeb64f59e18545030c7889951f6))
- Do not fail when encoder insertion try fails - ([3bdb41f](https://github.com/n0-computer/noq/commit/3bdb41fbfa59a88c29359bde7b235ae2e24d78e9))
- Guard against substract overflow - ([177b817](https://github.com/n0-computer/noq/commit/177b817591cf4b172d4ddebc464bddd831da98c2))
- Know if an index has been evicted, drop one by one - ([7b47683](https://github.com/n0-computer/noq/commit/7b47683347695cbc6d4d42680419730aac74eb20))
- Remove evicted fields from dynamic reference map - ([98a06b3](https://github.com/n0-computer/noq/commit/98a06b336fc071ef59fd58cdb08adfd0faa4cff3))
- Fix a prefi_int bug when integer encoding has a perfect fit - ([bdb3f55](https://github.com/n0-computer/noq/commit/bdb3f557acde26712388aba5ebebe6d24100e2f0))
- Fix 0 required ref case on encoding block - ([e56758d](https://github.com/n0-computer/noq/commit/e56758dcac623646ae726557d35f62405cba61da))
- Fix prefix string byte count when it fits 8 bit multiple - ([d62b5fe](https://github.com/n0-computer/noq/commit/d62b5fe06a4b3f222e7d9cbf78aecd3e5255c8b2))
- Rename HeaderBloc{,k}Field - ([b3263c3](https://github.com/n0-computer/noq/commit/b3263c38ecf48f631464ef291f301c5cc174acdd))
- Fix typo - ([550bf55](https://github.com/n0-computer/noq/commit/550bf55717b32a18dc02e521afd793a483b5106c))
- Remove dead_code - ([eca2af1](https://github.com/n0-computer/noq/commit/eca2af160459a57775a1f40a9c768754db4629cb))
- Fix visibilities - ([2fec389](https://github.com/n0-computer/noq/commit/2fec3894805dac1b5604ad7fd5976c592a3572f7))
- Use err_derive for public errors - ([dafe685](https://github.com/n0-computer/noq/commit/dafe6859955f6bde63127dc23589756076c1f73d))
- Fix default values for settings - ([ba2eff5](https://github.com/n0-computer/noq/commit/ba2eff5fe923c66809d92f3d6f5e6500b31c6591))

## [0.2.0](https://github.com/n0-computer/noq/compare/pre-quicr-quinn..0.2.0) - 2019-01-21

### Clippy

- :identical_conversion - ([80986de](https://github.com/n0-computer/noq/commit/80986de0a510ca4b0826c62cfa1399dc7da1e20b))
- :single_match - ([538154b](https://github.com/n0-computer/noq/commit/538154bc6d86f3338cb50f2b13c64bc50e3091e5))
- :collapsible_if - ([208a162](https://github.com/n0-computer/noq/commit/208a1622bccb85dd415a917b6cf8f1825dfdee40))
- :range_plus_one - ([c16c213](https://github.com/n0-computer/noq/commit/c16c2136c0deb235d29ccd2a1e6cb47e9e4f1b77))

## [pre-quicr-quinn](https://github.com/n0-computer/noq/compare/0.1.0..pre-quicr-quinn) - 2018-07-29

### ConnectionState

- :decode_key() can now be private - ([92e8c4d](https://github.com/n0-computer/noq/commit/92e8c4d06d9c6d7412e33ca754c5a1cab4998284))

## [pre-quinn-quicr](https://github.com/n0-computer/noq/compare/pre-quicr-quinn..pre-quinn-quicr) - 2018-07-29

### 📚 Documentation

- Typo fix - ([0a447c6](https://github.com/n0-computer/noq/commit/0a447c629d1fab48854c4e16bac16d17336fc6cf))

### ⚙️ Miscellaneous Tasks

- Specify quicr-core version - ([a256212](https://github.com/n0-computer/noq/commit/a25621234b9ed8bc2c925ef6725ccaed35ce750d))
- Fix high-latency handshakes and related bugs - ([31d3594](https://github.com/n0-computer/noq/commit/31d35944aacc8b52f436dca61b05de1a6c39db14))
- Fix 0-RTT send/recv - ([fb9190a](https://github.com/n0-computer/noq/commit/fb9190a0b6b07c9d8a39c943f4d290800e34f1a7))

### Endpoint

- :get_side - ([5b81b6a](https://github.com/n0-computer/noq/commit/5b81b6a5de8c77293c261dede92824c8a721fc8f))

### Core

- Clean up write - ([1c84faa](https://github.com/n0-computer/noq/commit/1c84faa0e57b36b2017bf6e55ceccd4b50b47ecf))
- Bitfield-based stream assembly - ([c0100f3](https://github.com/n0-computer/noq/commit/c0100f3c2af19cf70fdb67db5f539511b387e686))
- Implement ordered reads - ([b6cc9c2](https://github.com/n0-computer/noq/commit/b6cc9c2345a79727b933199de5ab1ef55edf9a74))
- Ensure read sanity - ([6f1ea7b](https://github.com/n0-computer/noq/commit/6f1ea7b783dd443fa88ec7703b98306616184adf))
- Truncate close reasons to fit MTU - ([e288af2](https://github.com/n0-computer/noq/commit/e288af23f76817d8136e735f4600e181af05be99))
- Fix panic on close - ([71ba828](https://github.com/n0-computer/noq/commit/71ba828c36362509bf8f836c6112727c6676ae06))
- Improve documentation - ([6d0254a](https://github.com/n0-computer/noq/commit/6d0254a0ad7bed9fea0ce2ff5e61b338e8f3b2d9))
- TLS certificate verification - ([b4bd5bc](https://github.com/n0-computer/noq/commit/b4bd5bc5517bd326b118cb8d25d40d24d047781d))
- Relax slog features - ([725fbd3](https://github.com/n0-computer/noq/commit/725fbd3874ede7805ff90b5761166e746cd80244))
- Extensive cleanup - ([8476e11](https://github.com/n0-computer/noq/commit/8476e117167a7e4f2718274039894a2cccf1bd17))
- Convenience impl From<ConnectionError> for io::Error - ([b8b0634](https://github.com/n0-computer/noq/commit/b8b063447ad3f67cd8837c6dadab1abb21d4f9e3))
- Fix client connection loss when Initial was retransmitted - ([fd69d56](https://github.com/n0-computer/noq/commit/fd69d565f68b177ca3f494f2215c9ae25b38207d))
- Support backpressure on incoming connections - ([63c4371](https://github.com/n0-computer/noq/commit/63c4371d99e3badf03834fee63e65df66438418a))
- Fix underflow on client handshake failure - ([d7754bf](https://github.com/n0-computer/noq/commit/d7754bfc31f412ffa39b6490e798d3b4f7045c17))
- Fix panic on stateless reset for short packets - ([71c9c48](https://github.com/n0-computer/noq/commit/71c9c482fb1415105358eacc378327a415510610))
- Test and debug stop_sending - ([ace87bb](https://github.com/n0-computer/noq/commit/ace87bbeec0941b60a8c727d4ab94acc28b49784))
- Deliver buffered data even after reset - ([f82774a](https://github.com/n0-computer/noq/commit/f82774a367249d78a2b0b111f313144b0aa66094))
- Test finishing streams - ([d0ee87f](https://github.com/n0-computer/noq/commit/d0ee87febaee67947c1bede77d12e965704d0ec7))
- Fix panic composing PNs before receiving ACKs - ([82016c4](https://github.com/n0-computer/noq/commit/82016c47a2d15e4bc08de92130fe9c810e4f7aa4))
- More detailed logging - ([6cef002](https://github.com/n0-computer/noq/commit/6cef002e07bf2d7fb8b01d81ed35e49f6f30d968))
- Fix default cert verification behavior - ([5d3aca1](https://github.com/n0-computer/noq/commit/5d3aca186ff7e0e71e13c35921156c20dec3d6e2))
- Unit test for congestion - ([934f681](https://github.com/n0-computer/noq/commit/934f681b77caa11622bfad4076d837b44e97d91e))
- Fix panic on long packet with empty payload - ([f1862b5](https://github.com/n0-computer/noq/commit/f1862b55fee9cd8b34786a9f54214d5f6997d6d5))
- Fix client bidi stream limit fencepost - ([a0b07dd](https://github.com/n0-computer/noq/commit/a0b07ddb7bfb06e79783ef28ea30dd3841de56d5))
- Fix connect test - ([aa5300d](https://github.com/n0-computer/noq/commit/aa5300da58a92b4ce59f9bb1010eb9f44d78d565))
- Log ACK delay - ([9ac732d](https://github.com/n0-computer/noq/commit/9ac732dd863d5a8f1ef1c3aa1f647baf1f573e45))
- Fix bad delay in server's first ACK - ([5e8dcc7](https://github.com/n0-computer/noq/commit/5e8dcc71ee669c750e06695ad10362ad5c2f4396))
- Fix inadvertent sending of certificate requests - ([1078ce9](https://github.com/n0-computer/noq/commit/1078ce9f76ce192a7dc57435890bee572eb7e637))
- Sni accessor - ([e40d241](https://github.com/n0-computer/noq/commit/e40d241c0de3670dd5bd69c4e670a12b83f7899e))
- Refactor tests to support passage of time - ([15662a3](https://github.com/n0-computer/noq/commit/15662a3f44ab5d10ee8909dfcfa5f5a1d285a621))
- Don't ignore handshake completion ACKs - ([6045f79](https://github.com/n0-computer/noq/commit/6045f79bd6b1469f979a159b26bc8a75370d42b4))
- Fix stream ID fencepost error - ([af50ca0](https://github.com/n0-computer/noq/commit/af50ca01549a91ab5256ef1b8959c02b9924e820))
- Fix underflow on recv of already-read stream frame - ([9e3467c](https://github.com/n0-computer/noq/commit/9e3467c0a28abe419025ae3e7009bf6173fc8b51))
- Fix panic on malformed header - ([0912eda](https://github.com/n0-computer/noq/commit/0912edaaf2a0ce390eb9a9b6a37a414a8e9ffbee))
- Fix openssl version bound - ([643c682](https://github.com/n0-computer/noq/commit/643c6829e9c00129f231ff6716978ab1823e5a56))
- Improve handling of unexpected long header packets - ([9892845](https://github.com/n0-computer/noq/commit/989284511d22e3bbcb7eda1099a7721c5fd3e56c))
- Tolerate NEW_CONNECTION_ID - ([d76f9e2](https://github.com/n0-computer/noq/commit/d76f9e2040b9af3c373bc5ed93695b1508b14c30))
- Sanity-check NEW_CONNECTION_ID - ([f65a59a](https://github.com/n0-computer/noq/commit/f65a59a22e70892b323c70f118159faa1d66f0a7))
- Optional stateless retry - ([55bf762](https://github.com/n0-computer/noq/commit/55bf7621a6c18cdb6a3d0a210fd48f786c879025))
- Minimal resumption UT - ([a598957](https://github.com/n0-computer/noq/commit/a5989575d2aad5714aa96b4f75e1fe8c5053d3c0))
- Ensure we don't use later TLS versions inadvertently - ([b0a3bc8](https://github.com/n0-computer/noq/commit/b0a3bc8837853d0f0406967eab4abe6708ab5a9e))
- Include TLS alert, if any, in handshake error - ([5e3e85a](https://github.com/n0-computer/noq/commit/5e3e85ab08cdc5325086401bef5260fef9b7308d))
- Fix incorrect retransmission of ClientHello under high latency - ([e6bd7ca](https://github.com/n0-computer/noq/commit/e6bd7cad9ca4a0c22872ae3531dbdba4018dbefa))
- Fix server dedup of retransmitted Initial under stateful hs - ([9f13021](https://github.com/n0-computer/noq/commit/9f130218413de7e467d07fea8f8e97ea8d6a3e61))
- Don't send MAX_DATA after handshake - ([8ccd95c](https://github.com/n0-computer/noq/commit/8ccd95c7033f3bb3e1be4d6de73119321e6934d8))
- Clarify some errors - ([e65eaa2](https://github.com/n0-computer/noq/commit/e65eaa2ea3a012dfeff163daca11bae91abf6e7a))
- Don't inspect reserved short header bit - ([1f1f946](https://github.com/n0-computer/noq/commit/1f1f9463fe102385578d2388cc4e9a2c8b6e02e8))
- Remove dead code - ([14f33b7](https://github.com/n0-computer/noq/commit/14f33b74be6c2b792e1520f479d82956162fa5f2))
- Draft 0-RTT receive support - ([7c7f635](https://github.com/n0-computer/noq/commit/7c7f63552749c73dcaedd77e4dcc9ce3e742ac48))
- Draft 0-RTT transmit support - ([5993415](https://github.com/n0-computer/noq/commit/5993415434ff8a91cd4447debe87dbfc7198875b))
- Allow ACK-only handshake packets - ([a9f9ec6](https://github.com/n0-computer/noq/commit/a9f9ec614ddb485533f3835ddc94431bd77ca62c))
- Optional stateless reset token, fix CID spoofing attack - ([a53a0b3](https://github.com/n0-computer/noq/commit/a53a0b384c0ace3cf8f920c27d5749de406cc22c))
- Only report stateless resets once - ([4b01245](https://github.com/n0-computer/noq/commit/4b0124518760460810155496b5ff92f07ef33903))
- Update for current rust-openssl - ([4cecc71](https://github.com/n0-computer/noq/commit/4cecc71a8608ac76857fd3ba207688bea0329382))

### Examples

- Support fetching arbitrary URLs - ([94f3c63](https://github.com/n0-computer/noq/commit/94f3c63959acbfd582e7c71077e6c086edc23567))
- Disable certificate verification in client - ([1930534](https://github.com/n0-computer/noq/commit/19305344a3c22bcbdbeb83f7d20e90c8265c438f))
- Richer logging - ([25c48a2](https://github.com/n0-computer/noq/commit/25c48a29d3b9a603f49bc94a36bbec058c0ed3bf))
- Server: configurable PEM certs - ([36b627b](https://github.com/n0-computer/noq/commit/36b627b8da3bf665a874b1a2c9a7a04860b4ab52))
- Use packaged single-threaded runtime - ([7b5499f](https://github.com/n0-computer/noq/commit/7b5499f1e77832bf8217c26ea86c197cae98c76e))
- Less monolithic server future - ([a4fbb44](https://github.com/n0-computer/noq/commit/a4fbb443ea4934839c92495c2d1c17e156ffbec3))
- Mark unreachable case - ([b44a612](https://github.com/n0-computer/noq/commit/b44a612304b7f5826a76cfcdc6934ee496fb1daf))
- Expose stateless retry - ([843fc3c](https://github.com/n0-computer/noq/commit/843fc3c89aace5e7c3509cf57c9c2a1c8e2af9f7))
- Allow arbitrary listen address - ([5ca79bb](https://github.com/n0-computer/noq/commit/5ca79bb6b99564bc91e919e72cacddecc684ced2))

### Nix

- Always have backtraces - ([f2d88da](https://github.com/n0-computer/noq/commit/f2d88da66b40a1ab1151e0595e5cf0efe601ffe3))

### Readme

- Badge tweaks - ([53c4156](https://github.com/n0-computer/noq/commit/53c4156c203d5f6d8a75062c7eef13a99345085e))
- API docs link - ([a90da18](https://github.com/n0-computer/noq/commit/a90da181bf6fbe075994c53733c035159b305d2e))

### Shell

- Use an OpenSSL capable of logging exporter secrets - ([40b4a59](https://github.com/n0-computer/noq/commit/40b4a59390a314555006d9fb7d9113d50c343477))

### Tokio

- Separate send/recv stream types - ([3d30b10](https://github.com/n0-computer/noq/commit/3d30b104b4213c964a9013ecde6eb9b0772a1253))
- Fix panic on connection loss - ([69cf450](https://github.com/n0-computer/noq/commit/69cf45062ab5049231b6607811753eb5281e9665))
- Impl AsyncRead for RecvStream - ([73d9e34](https://github.com/n0-computer/noq/commit/73d9e3470ed2a260e3b03694241cbca1750f7957))
- Refactor and document API - ([08756e4](https://github.com/n0-computer/noq/commit/08756e4e6ddaa166aaa66c1abd1c13c372a51c41))
- Endpoint builder - ([3fc7535](https://github.com/n0-computer/noq/commit/3fc75350644cc111cf3ee8d502b2974620310e63))
- Ergonomics and documentation - ([57ef2f6](https://github.com/n0-computer/noq/commit/57ef2f68fbb55bad95f257016386a641ca55a20b))
- Doc fix - ([ba19a86](https://github.com/n0-computer/noq/commit/ba19a865bbf32c2bff29b55c55e4f5e0805ad628))
- Graceful close - ([68e0db5](https://github.com/n0-computer/noq/commit/68e0db51a4003c8cc315eecc7ba34ecc6779d763))
- Expose API for STOP_SENDING - ([3e72bc9](https://github.com/n0-computer/noq/commit/3e72bc9cee69f0e994da88bad0b70bcd5d296530))
- Docs link - ([64a8d46](https://github.com/n0-computer/noq/commit/64a8d46c4026021235ace79379a3e229db6063e7))
- Update for rustc 1.26 - ([6160a53](https://github.com/n0-computer/noq/commit/6160a53625394d35d2e40fac9d6220d489dd099c))
- Work around panic on handshake failure - ([72c9e4b](https://github.com/n0-computer/noq/commit/72c9e4be350076f7335b08140c2766c5e47e80da))
- Expose 0-RTT writes - ([0a93bf4](https://github.com/n0-computer/noq/commit/0a93bf4bda94cfcdd570502d7653a523b9ad34ae))
- Fix stateless reset handling - ([897b804](https://github.com/n0-computer/noq/commit/897b804d96df749b6a7e3ccc629496e450c558f6))

### Transport_parameters

- :Error: Fail - ([a69dd0b](https://github.com/n0-computer/noq/commit/a69dd0bd0193f2b8ea2580422e509fb34c72daa7))


