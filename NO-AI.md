# No AI, no network

Pinwheel contains no machine-learning models and makes no network connections. Both promises are checked automatically on every build. Please read this before contributing.

## The rules

1. **No machine learning.** No neural networks, no trained models, no model files, and no dependencies that ship or run them. Every result must come from a classic, explainable algorithm.
2. **No network.** Pinwheel never connects to the internet: no update checks, no downloads, no accounts, no telemetry, no crash uploads. Crash logs stay on the computer; users attach them to an issue themselves if they want.
3. **Predictable output.** The same file with the same settings gives the same result, so anyone can verify a build.

## What this rules out, and what we do instead

| Common "smart" feature | Why it is out | Pinwheel's classic alternative |
|---|---|---|
| AI background removal | Needs a neural network | Colour-key removal for plain backgrounds (signatures, logos, product shots on white) |
| AI upscaling | Needs a neural network | Honest resampling with a warning past 200% |
| OCR (text from images) | Modern Tesseract uses an LSTM network | Text is extracted only from PDFs that already contain it |
| Smart or face-aware crop | Needs detection models | User drags the focal point |
| AI auto-enhance | Needs a model | Auto levels from the histogram, fully predictable |

## How it is enforced

- **Dependency check in CI.** The build fails if any dependency matches the deny list in [`scripts/no-ai-check.sh`](scripts/no-ai-check.sh), such as onnxruntime, TensorFlow, PyTorch, Tesseract or OpenCV's DNN module.
- **Model-file check in CI.** The build fails if the repository or a release contains model files such as `.onnx`, `.tflite`, `.pt` or `.gguf`.
- **Network crates are denied too.** HTTP clients and socket libraries (reqwest, hyper, ureq, …) are on the same deny list.
- **No network, verifiably.** On macOS the sandbox has no network permission; on Linux the Flatpak has no network access; on Windows the code contains no networking and the end-to-end tests run with networking blocked.

If you think a dependency is wrongly flagged, open an issue explaining what it does. Do not weaken the check in the same pull request that adds the dependency.
