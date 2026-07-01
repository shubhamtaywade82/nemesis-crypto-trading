#!/usr/bin/env python3
"""Generate Python protobuf stubs from canonical proto definitions."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
PROTO_DIR = ROOT / "proto"
OUT_DIR = Path(__file__).resolve().parent.parent / "src" / "nemesis_alpha" / "proto"

def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    cmd = [
        sys.executable, "-m", "grpc_tools.protoc",
        f"--proto_path={PROTO_DIR}",
        f"--python_out={OUT_DIR}",
        f"--pyi_out={OUT_DIR}",
        str(PROTO_DIR / "envelope.proto"),
    ]

    print(f"Generating protos: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"ERROR: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    init_file = OUT_DIR / "__init__.py"
    if not init_file.exists():
        init_file.write_text("# Auto-generated protobuf package\n")

    print(f"Generated protos in {OUT_DIR}")

if __name__ == "__main__":
    main()
