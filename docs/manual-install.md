# Manual Install

Use this when Vera cannot download ONNX Runtime or model files directly, for example on corporate networks that only allow downloads in a browser.

## 1. Download The Files In A Browser

For the default local stack, download:

- ONNX Runtime for your backend from the [Microsoft ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)
- [`jinaai/jina-embeddings-v5-text-nano-retrieval`](https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval)
- [`jinaai/jina-reranker-v2-base-multilingual`](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual)

If you want the optional CodeRankEmbed preset instead of Jina embeddings, download:

- [`Zenabius/CodeRankEmbed-onnx`](https://huggingface.co/Zenabius/CodeRankEmbed-onnx)
- [`jinaai/jina-reranker-v2-base-multilingual`](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual)

## 2. Place Them Under `~/.vera/`

Vera looks for these paths by default:

| Asset | Destination |
| --- | --- |
| ONNX Runtime CPU | `~/.vera/lib/` |
| ONNX Runtime GPU backend | `~/.vera/lib/<backend>/` such as `~/.vera/lib/cuda/` |
| Jina embeddings | `~/.vera/models/jinaai/jina-embeddings-v5-text-nano-retrieval/` |
| CodeRankEmbed | `~/.vera/models/Zenabius/CodeRankEmbed-onnx/` |
| Local reranker | `~/.vera/models/jinaai/jina-reranker-v2-base-multilingual/` |

Expected filenames for the curated presets:

| Model | Files |
| --- | --- |
| Jina embeddings | `onnx/model_quantized.onnx`, `onnx/model_quantized.onnx_data`, `tokenizer.json` |
| CodeRankEmbed | `onnx/model_quantized.onnx`, `tokenizer.json` |
| Jina reranker | `onnx/model_quantized.onnx`, `tokenizer.json` |

If you want to keep a custom embedding model somewhere else, skip copying it into `~/.vera/models/` and point Vera at it directly with `vera setup --embedding-dir /path/to/model-dir`.

## 3. Re-run Setup Or Repair

```bash
vera setup --onnx-jina-cuda
# or
vera setup --onnx-jina-cuda --code-rank-embed
# or
vera repair --onnx-jina-cuda
```

Then verify:

```bash
vera doctor
vera doctor --probe
```

## Notes

- Set `VERA_HOME` if you want Vera to use a different base directory than `~/.vera`.
- Set `ORT_DYLIB_PATH` if you installed ONNX Runtime somewhere else and want Vera to use that exact shared library.
- On Windows CUDA 13, the ONNX Runtime archive name and the folder inside the archive do not match. Current Vera releases handle that layout correctly.
