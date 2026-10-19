#!/usr/bin/env bash
# Enforces NO-AI.md: fails if any dependency can run a machine-learning model
# or talk to the network, or if a model file is present.
#
# Usage: scripts/no-ai-check.sh [DIR...]
#   With no arguments, checks Cargo.lock and every file tracked by git.
#   With directories (such as an unpacked release), also scans those.
set -euo pipefail
cd "$(dirname "$0")/.."

# Crates that ship or run ML models.
ml='onnxruntime.*|ort|ort-sys|tensorflow.*|tflite.*|tch|torch.*|candle-.*|burn|burn-.*|tract-.*|tesseract.*|leptess|opencv.*|llama.*|ggml.*|whisper.*|rust-bert|tokenizers|linfa.*|smartcore|dfdx|mediapipe.*'
# Crates that make network connections.
net='reqwest|hyper|hyper-.*|ureq|curl|curl-sys|isahc|surf|attohttpc|h2|h3|quinn.*|rustls|native-tls|tokio-tungstenite|tungstenite|websocket.*'

status=0

if [[ -f Cargo.lock ]]; then
  names=$(sed -n 's/^name = "\(.*\)"$/\1/p' Cargo.lock | sort -u)
  bad=$(grep -xE "$ml|$net" <<<"$names" || true)
  if [[ -n "$bad" ]]; then
    echo "Denied dependencies (see NO-AI.md):"
    sed 's/^/  /' <<<"$bad"
    status=1
  fi
fi

models='\.(onnx|ort|tflite|pt|pth|ckpt|safetensors|gguf|ggml|pb|h5|keras|mlmodel|mlpackage|caffemodel|traineddata)$'
found=$(git ls-files | grep -iE "$models" || true)
for dir in "$@"; do
  found+=$'\n'$(find "$dir" -type f | grep -iE "$models" || true)
done
found=$(sed '/^$/d' <<<"$found")
if [[ -n "$found" ]]; then
  echo "Model files are not allowed (see NO-AI.md):"
  sed 's/^/  /' <<<"$found"
  status=1
fi

if [[ $status -eq 0 ]]; then
  echo "No-AI check passed: no ML or network dependencies, no model files."
fi
exit $status
