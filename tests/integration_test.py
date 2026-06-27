import os
import subprocess
import time

def test_boot_artifacts():
    print("Verifying build artifacts...")
    artifacts = [
        "build/initramfs.cpio.gz",
        "target/release/ayux_init",
        "target/release/login_manager",
        "target/release/ayux_shell"
    ]
    for art in artifacts:
        if not os.path.exists(art):
            print(f"FAIL: Artifact {art} not found")
            return False
    print("All required artifacts are present.")
    return True

def test_rootfs_structure():
    print("Verifying rootfs structure...")
    rootfs_bin = "build/rootfs/bin"
    required_bins = ["login_manager", "ayux_shell"]
    for b in required_bins:
        if not os.path.exists(os.path.join(rootfs_bin, b)):
            print(f"FAIL: Binary {b} missing from rootfs/bin")
            return False

    if not os.path.exists("build/rootfs/init"):
        print("FAIL: init (ayux_init) missing from rootfs root")
        return False

    print("Rootfs structure looks correct.")
    return True

if __name__ == "__main__":
    success = test_boot_artifacts() and test_rootfs_structure()
    if success:
        print("Integration tests (Verification mode) PASSED")
    else:
        print("Integration tests FAILED")
        exit(1)
