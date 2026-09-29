"""Prebuilt libclang for rust_bindgen.

The default rules_rust bindgen toolchain builds llvm-project from source.
Those genrules call python3, which the ubuntu:22.04 executor image does not
have. This repo rule unpacks the shared libraries bindgen dlopens instead.
"""

_UBUNTU = "http://archive.ubuntu.com/ubuntu/"
_LLVM_APT = "https://apt.llvm.org/jammy/"

# (name, url, sha256)
_DEBS = [
    (
        "libclang",
        _LLVM_APT + "pool/main/l/llvm-toolchain-22/libclang1-22_22.1.8~++20260613092327+e80beda6e255-1~exp1~20260613092437.81_amd64.deb",
        "00c093f3028f821df6272d74b66bdcd4a8db474d1b4d22769a0ba166d5864e79",
    ),
    (
        "libllvm",
        _LLVM_APT + "pool/main/l/llvm-toolchain-22/libllvm22_22.1.8~++20260613092327+e80beda6e255-1~exp1~20260613092437.81_amd64.deb",
        "4bd592cab11e90e5b8625b5e3a1ceb5936e843c63bf7394f6f6506aa41302164",
    ),
    (
        "libffi8",
        _UBUNTU + "pool/main/libf/libffi/libffi8_3.4.2-4_amd64.deb",
        "b4f88c91fa6f4c942097be6abfc365fb133c5e147640168cbb7704fd855eac10",
    ),
    (
        "libedit2",
        _UBUNTU + "pool/main/libe/libedit/libedit2_3.1-20210910-1build1_amd64.deb",
        "fb8783bdf0a59aaabf4e9b29192170f6cc17aa2eb94bc672c15add37ff39525b",
    ),
    (
        "libbsd0",
        _UBUNTU + "pool/main/libb/libbsd/libbsd0_0.11.5-1_amd64.deb",
        "09367acdf59f28ffb71ba7bc36c516cff03b2765930cf9be7bb7ea7273e4c312",
    ),
    (
        "libmd0",
        _UBUNTU + "pool/main/libm/libmd/libmd0_1.0.4-1build1_amd64.deb",
        "bcdad5e400aeb660f0337a1c5c1a77e3effa91d0803e64ce2d000e9006f67cdb",
    ),
    (
        "libtinfo6",
        _UBUNTU + "pool/main/n/ncurses/libtinfo6_6.3-2ubuntu0.3_amd64.deb",
        "3b8dff43a31168f5020928f2ab9a97324c5b47ae708311781f6bd01306c96214",
    ),
    (
        "libz3",
        _UBUNTU + "pool/universe/z/z3/libz3-4_4.8.12-1_amd64.deb",
        "d83fda4b2e58d9b30f0f85b02a3eb48ca7d33ca88e9d4584249e698bc2e451cf",
    ),
    (
        "zlib1g",
        _UBUNTU + "pool/main/z/zlib/zlib1g_1.2.11.dfsg-2ubuntu9.2_amd64.deb",
        "9dc17e51a1be2d9ed63b7b84ef0e4e29c5abe6f1bc62cb03e7181483cce8a2f2",
    ),
    (
        "libzstd1",
        _UBUNTU + "pool/main/libz/libzstd/libzstd1_1.4.8+dfsg-3build1_amd64.deb",
        "ae7db00ce8b093e50c994518b90203544e063b4bc574836a048bb142b950b2c9",
    ),
    (
        "libxml2",
        _UBUNTU + "pool/main/libx/libxml2/libxml2_2.9.13+dfsg-1ubuntu0.13_amd64.deb",
        "825df4a2c852b4cec058ed3e95648f5c7b41f923af85e15d464ec80c14302edc",
    ),
    (
        "liblzma5",
        _UBUNTU + "pool/main/x/xz-utils/liblzma5_5.2.5-2ubuntu1.1_amd64.deb",
        "af77f099e7fa95a31267023535832804382fa66572a6b38002187be6e15116cb",
    ),
    (
        "libicu70",
        _UBUNTU + "pool/main/i/icu/libicu70_70.1-2_amd64.deb",
        "58a154f6307289813da2276f900498ef536ae7c0522d2cf31a3c3c5cf62dfd9a",
    ),
    (
        "libstdcxx",
        _UBUNTU + "pool/main/g/gcc-12/libstdc++6_12.3.0-1ubuntu1~22.04.3_amd64.deb",
        "29ef3d289b272704b75141906c8f683db0f2bdb0942061366691333bfb289bdc",
    ),
    (
        "libgcc",
        _UBUNTU + "pool/main/g/gcc-12/libgcc-s1_12.3.0-1ubuntu1~22.04.3_amd64.deb",
        "d383e642d83263147f7b4b69fb92cf1036e5246c6be3accf10b77f986684589a",
    ),
]

# (deb name, path inside the deb, soname written under lib/)
# libclang and libLLVM use RUNPATH $ORIGIN/../lib, so every NEEDED soname
# has to sit in that same directory. LD_LIBRARY_PATH covers the rest.
_COPIES = [
    ("libclang", "usr/lib/x86_64-linux-gnu/libclang-22.so.22", "libclang-22.so.22"),
    ("libllvm", "usr/lib/x86_64-linux-gnu/libLLVM.so.22.1", "libLLVM.so.22.1"),
    ("libffi8", "usr/lib/x86_64-linux-gnu/libffi.so.8", "libffi.so.8"),
    ("libedit2", "usr/lib/x86_64-linux-gnu/libedit.so.2", "libedit.so.2"),
    ("libbsd0", "usr/lib/x86_64-linux-gnu/libbsd.so.0", "libbsd.so.0"),
    ("libmd0", "usr/lib/x86_64-linux-gnu/libmd.so.0", "libmd.so.0"),
    ("libtinfo6", "lib/x86_64-linux-gnu/libtinfo.so.6", "libtinfo.so.6"),
    ("libz3", "usr/lib/x86_64-linux-gnu/libz3.so.4", "libz3.so.4"),
    ("zlib1g", "lib/x86_64-linux-gnu/libz.so.1", "libz.so.1"),
    ("libzstd1", "usr/lib/x86_64-linux-gnu/libzstd.so.1", "libzstd.so.1"),
    ("libxml2", "usr/lib/x86_64-linux-gnu/libxml2.so.2", "libxml2.so.2"),
    ("liblzma5", "lib/x86_64-linux-gnu/liblzma.so.5", "liblzma.so.5"),
    ("libicu70", "usr/lib/x86_64-linux-gnu/libicuuc.so.70", "libicuuc.so.70"),
    ("libicu70", "usr/lib/x86_64-linux-gnu/libicudata.so.70", "libicudata.so.70"),
    ("libstdcxx", "usr/lib/x86_64-linux-gnu/libstdc++.so.6", "libstdc++.so.6"),
    ("libgcc", "lib/x86_64-linux-gnu/libgcc_s.so.1", "libgcc_s.so.1"),
]

def _target_name(soname):
    return "so_" + soname.replace("+", "x").replace("-", "_").replace(".", "_")

def _llvm_prebuilt_impl(repository_ctx):
    for name, url, sha256 in _DEBS:
        repository_ctx.download(
            url = url,
            sha256 = sha256,
            output = "debs/%s.deb" % name,
        )

    lines = [
        "set -eu",
        "rm -rf extract lib",
        "mkdir -p extract lib",
    ]
    for name, _, _ in _DEBS:
        lines.append("dpkg-deb -x debs/%s.deb extract" % name)
    for _, src, dest in _COPIES:
        lines.append("cp -L extract/%s lib/%s" % (src, dest))
        lines.append("sig=$(od -An -t x1 -N 4 lib/%s | tr -d ' \\n')" % dest)
        lines.append("test \"$sig\" = 7f454c46")
    lines.append("rm -rf extract debs")
    repository_ctx.file("unpack.sh", "\n".join(lines) + "\n", executable = True)
    result = repository_ctx.execute(
        ["bash", "unpack.sh"],
        timeout = 600,
        working_directory = str(repository_ctx.path(".")),
    )
    if result.return_code != 0:
        fail("llvm_prebuilt unpack failed:\n%s\n%s" % (result.stdout, result.stderr))

    imports = []
    deps = []
    seen = {}
    for _, _, dest in _COPIES:
        target = _target_name(dest)
        if target in seen:
            continue
        seen[target] = True
        imports.append(
            "cc_import(\n    name = \"%s\",\n    shared_library = \"lib/%s\",\n)" % (target, dest),
        )
        deps.append("        \":%s\"," % target)

    repository_ctx.file("BUILD.bazel", """load("@rules_cc//cc:defs.bzl", "cc_import", "cc_library")

package(default_visibility = ["//visibility:public"])

%s

cc_library(
    name = "libclang",
    deps = [
%s
    ],
)
""" % ("\n\n".join(imports), "\n".join(deps)))

llvm_prebuilt = repository_rule(
    implementation = _llvm_prebuilt_impl,
    doc = "x86_64 libclang 22 and the shared libraries it dlopens, for the exec platform.",
)
