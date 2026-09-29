# Process

Build a Rust program with Bazel, remotely, and run it on `orin-1`. The program links `llama-cpp-2`. The CUDA build of llama.cpp, compiled from the sources vendored in `llama-cpp-sys-2`, is the only native target.

The workstation does not compile. `orin-1` does not compile. BuildBuddy compiles, on x86_64 executors, and the output is an aarch64 binary. That binary is copied over SSH to the Orin and executed there.

## Standing decisions

- The workstation does not run Bazel, `bazel mod tidy`, Cargo, or anything else that fetches modules or crates. BuildBuddy is the only place that resolves and compiles, and only after the commit is on the remote. Every `bb` invocation passes `--config=buildbuddy`. Do not pass `--arch=arm64`.
- The build system is Bazel. `rules_cuda` compiles the `.cu` files. `cc_library` compiles the C and C++. `rules_rust` compiles the Rust and links the native libraries.
- The product target is the CUDA binary. Device architecture is `sm_87` only.
- `llama-cpp-sys-2` does not run a Cargo build script. `rust_bindgen` generates the FFI.
- The CUDA redistributable is Linux SBSA, pinned to CUDA 13.2, matching driver 595 on this board.
- The executor pool is the existing x86_64 BuildBuddy pool. The runner is amd64. The target platform makes the binary aarch64.

## The device

`orin-1@orin-1.local` on the LAN. The name `orin-1` does not resolve. SSH to `orin-1.local` works. The password file is `/mnt/workspace/orin-1.key` and stays out of this repo.

| | |
|---|---|
| Board | Jetson Orin Nano, L4T R39 revision 2.1, kernel `6.8.12-1021-tegra` |
| OS | Ubuntu 24.04.4, glibc 2.39 |
| Driver | 595.78, `nvidia-smi` reports CUDA 13.2 |
| GPU | `Orin (nvgpu)`, compute capability 8.7, 8 SMs |
| CPU | Cortex-A78AE, Armv8.2-A with dotprod and FP16. No I8MM. |
| Memory | 7.3 GiB unified. With a desktop session, free memory is about 3.7 GiB. |
| Disk | about 65 GB free on `/` |
| Userspace CUDA | driver libs under `/opt/nvidia/l4t-gpu-libs/nvgpu/` and `/lib/aarch64-linux-gnu/`. No `/usr/local/cuda`. |

JetPack 7.2 is what lets this board run the ARM SBSA CUDA userspace on the nvgpu driver. That is the userspace the binary links. The binary dlopens the board's own `libcuda.so.1` at run time. No compat `libcuda` is shipped.

## Roles

```text
workstation
  push a commit
  bb remote --os=linux
        |
        v
BuildBuddy runner (Ubuntu 22.04, x86_64)          the Bazel client
        |
        |  --extra_execution_platforms=//:rbe
        v
BuildBuddy executors (docker://ubuntu:22.04, x86_64)
  rustc, hermetic clang, and nvcc actions
  target platform linux_arm64, device arch sm_87
        |
        |  tar the binary and its runfiles into
        |  $BUILDBUDDY_ARTIFACTS_DIRECTORY
        v
workstation downloads the archive
  rsync to orin-1@orin-1.local
  the Orin executes the binary
```

`bb` reads its API key from the repo-local git config key `buildbuddy.api-key`. That value is not a file in the tree. The remote build runs from a commit that is already on GitHub. BuildBuddy checks that commit out itself.

```bash
bb remote \
  --os=linux \
  --run_from_commit=<sha> \
  --timeout=3h \
  --runner_exec_properties=recycle-runner=false \
  --runner_exec_properties=EstimatedFreeDiskBytes=80GB \
  build \
  --config=buildbuddy \
  --config=release \
  //:<binary>
```

`--arch=arm64` fails: there is no Linux arm64 pool in the workflows executor set. Leave `--arch` unset. The runner is amd64. The binary is aarch64 because `--platforms` says so.

`--remote_download_minimal` returns the top-level ELF to the client and leaves the runfiles on the runner. The runner packs both, following symlinks:

```bash
bazel build --config=buildbuddy --config=release --remote_download_toplevel //:<binary>
tar -C bazel-bin -czhf "$BUILDBUDDY_ARTIFACTS_DIRECTORY/<binary>.tar.gz" \
  <binary> <binary>.runfiles
```

Download the bytestream blob from the runner invocation, `rsync` it to `orin-1@orin-1.local`, unpack it so the executable and the runfiles directory are siblings, and run it there. Static cuBLAS means the Orin does not receive a CUDA toolkit. The dynamic linker on the board resolves `libcuda.so.1`.

## Platforms

Defined in this repo. No external project labels.

Target, `//:linux_arm64`:

- `@platforms//cpu:aarch64`
- `@platforms//os:linux`
- `@llvm//constraints/libc:gnu.2.28`

The glibc floor is 2.28 so the binary runs on the Orin's 2.39.

Execution, `//:rbe`:

- `@platforms//cpu:x86_64`
- `@platforms//os:linux`
- `@llvm//constraints/libc:gnu.2.28`
- `@bazel_tools//tools/cpp:clang`
- `exec_properties`: `container-image=docker://ubuntu:22.04`, `OSFamily=Linux`

`.bazelrc` selects them and carries the BuildBuddy config:

```text
common --platforms=//:linux_arm64
common --extra_execution_platforms=//:rbe
common --compiler=clang

common --@rules_cuda//cuda:archs=sm_87
common --@rules_cuda//cuda:compiler=nvcc
common --@rules_cuda//cuda:runtime=@cuda//:cuda_runtime_static

common:buildbuddy --bes_results_url=https://app.buildbuddy.io/invocation/
common:buildbuddy --bes_backend=grpcs://remote.buildbuddy.io
common:buildbuddy --remote_cache=grpcs://remote.buildbuddy.io
common:buildbuddy --experimental_remote_downloader=grpcs://remote.buildbuddy.io
common:buildbuddy --remote_executor=grpcs://remote.buildbuddy.io
common:buildbuddy --remote_timeout=3600
common:buildbuddy --remote_cache_compression
common:buildbuddy --rewind_lost_inputs
common:buildbuddy --experimental_remote_downloader_local_fallback
common:buildbuddy --grpc_keepalive_time=30s
common:buildbuddy --jobs=80
common:buildbuddy --extra_execution_platforms=//:rbe
common:buildbuddy --experimental_repo_remote_exec=true
common:buildbuddy --remote_upload_local_results
common:buildbuddy --remote_download_minimal
```

If nvcc rejects the hermetic clang as too new, enable the `nvcc_allow_unsupported_compiler` feature. That flag is the fix. The clang toolchain stays the one selected above.

## Modules

- `rules_rust`, with rustc 1.98 and both `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu` in `crate.from_cargo`'s `supported_platform_triples`. The x86_64 rustc emits aarch64 objects and links them with the aarch64 clang.
- `rules_rust_bindgen`, for the FFI action. The registered toolchain is `//third_party/bindgen:bindgen_toolchain`. It dlopens a prebuilt libclang 22 (`third_party/llvm_prebuilt.bzl`). The module's default toolchain builds `@llvm-project` from source, and that path calls `python3`, which the ubuntu:22.04 executor image does not have. Hermetic `@llvm` stays the C++ compiler; its prebuilt ships `clang` and not `libclang.so`.
- `rules_cc`.
- `llvm`, hermetic clang, the same cross toolchain that already targets aarch64 at the glibc 2.28 floor. Registered with `@llvm//toolchain:all`.
- `rules_cuda`. `cuda.redist_json` for CUDA 13.2, platforms `linux-x86_64` and `linux-sbsa`. `@rules_cuda//cuda:aarch64` stays `sbsa`.
- `platforms`.

`linux-x86_64` of the redistributable is nvcc, ptxas, and cicc, and those run on the executors. `linux-sbsa` is the headers and the static libraries linked into the aarch64 binary. nvcc's host compiler (`-ccbin`) is the aarch64 hermetic clang. Device code is `sm_87`.

## The native graph

Sources are the `llama.cpp` tree vendored inside `llama-cpp-sys-2`. An additive `BUILD` file on that crate defines the targets, so the headers `rust_bindgen` parses and the code nvcc compiles are the same revision.

| Target | Rule | Contents |
|---|---|---|
| `ggml-base` | `cc_library` | `ggml.c`, `ggml.cpp`, alloc, backend, quants, gguf |
| `ggml-cpu` | `cc_library` | the CPU backend this CUDA build links, compiled `-march=armv8.2-a+dotprod+fp16` |
| `ggml-cuda` | `cuda_library` | `ggml-cuda/*.cu` and the checked-in FlashAttention template instances |
| `ggml` | `cc_library` | backend registration, public define `GGML_USE_CUDA` |
| `llama` | `cc_library` | `src/*.cpp` |
| `llama-common` | `cc_library` | `common/*.cpp` and `wrapper_common.cpp` |

`ggml-cpu` is a dependency of this CUDA target. ggml's scheduler and the ops that have no CUDA kernel live there. It is part of the CUDA binary.

`ggml-cuda` defines `GGML_USE_CUDA`, `GGML_CUDA_NO_VMM`, and `GGML_CUDA_USE_GRAPHS`. `GGML_CUDA_NO_VMM` is the Orin setting: unified memory, and the link line does not contain `libcuda`. OpenMP is absent. NCCL is absent.

The FlashAttention files under `ggml-cuda/template-instances/` are already in the tree. The `cuda_library` lists the same instances llama.cpp's CUDA build compiles, and it sets the matching `GGML_CUDA_FA_*` defines, because `fattn.cu` references those symbols. The Python generator that wrote those files is not a build action.

`ggml-version.h` is a Bazel `expand_template` of the `ggml-version.h.in` already in the tree.

`ggml-cuda` depends on the static SBSA targets rules_cuda generates for `cudart`, `cublas`, `cublasLt`, and `culibos`. `--@rules_cuda//cuda:runtime` selects the static CUDA runtime.

Each `.cu` is its own Bazel action. BuildBuddy caches them and runs them in parallel on the x86_64 executors. The FlashAttention instances are the long actions, and they are cached individually.

## Rust

`crate.from_specs` resolves `llama-cpp-2` 0.1.157 on the BuildBuddy runner. There is no `Cargo.lock` and no crate-universe lockfile in this repo, because generating either one fetches crates on the machine that runs Bazel.

`llama-cpp-sys-2` 0.1.157 (`sha256` `cdd0b39fc874267312983364d48379179a7945ad9d4c068c176304ae1cdca1ec`) is an `http_archive`. Its `BUILD` file is `third_party/llama/BUILD.llama.bazel`. The vendored llama.cpp snapshot is `26394b4`.

On `llama-cpp-sys-2`:

- `gen_build_script = "off"`. The crate's `build.rs` is not a Bazel action.
- `crate_features = ["cuda", "cuda-no-vmm", "common"]`. `llama-cpp-2` gates Rust code on these features. The features do not drive a native build.
- `link_deps` is `llama-common`, which links `llama`, `ggml`, `ggml-cuda`, and the SBSA static CUDA libraries.

`rules_cuda` is git commit `8dd68e7ade0c681d46eb659e662e9b4fe1e87013`. BCR 0.3.0 cannot fetch `linux-sbsa`. A patch adds `cublas_static` and `cublasLt_static`. CUDA is 13.2.0, components `cccl`, `crt`, `cudart`, `culibos`, `cublas`, `nvcc`, `nvvm`, and `nvjitlink`.

`rust_bindgen` compiles `wrapper.h` with the sys crate's allowlist (`ggml_*`, `gguf_*`, `llama_*`, and `llama_rs_*` when `common` is on). A patch to the sys crate includes that generated file instead of the `OUT_DIR` path `build.rs` used to write.

`llama-cpp-2` depends on that sys crate. The application is a `rust_binary` on top of `llama-cpp-2`.

## Run

Unpack the archive on `orin-1` so the executable and `<binary>.runfiles` are siblings. The dynamic linker finds `libcuda.so.1` from the nvgpu driver paths above. The first execution loads a GGUF that fits in the free unified memory, with GPU layers enabled.
