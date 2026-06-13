# Bundled model — license & attribution

## `movenet-lightning.onnx`

- **Model:** MoveNet SinglePose Lightning (fp32), COCO-17 keypoints.
- **Author:** Google Research (original TensorFlow model).
- **ONNX export:** `Xenova/movenet-singlepose-lightning`
  (<https://huggingface.co/Xenova/movenet-singlepose-lightning>), `onnx/model.onnx`,
  produced via `tf2onnx`.
- **License:** **Apache License 2.0** — the license grant applies to the model
  artifact itself. Full text: <https://www.apache.org/licenses/LICENSE-2.0>.

This file is **reference data**, committed so the build is reproducible and
offline (`include_bytes!`, SPEC-0013 §2.6). It is the matching **prior**'s
pose-feature extractor (R-0013); it is never retrained here.

> Note on fp32 vs fp16: the fp16 export emits incorrect keypoints under ONNX
> Runtime's CPU kernels, so the fp32 export is bundled (SPEC-0013 §2.6 realization).

### Apache 2.0 — required notices

```
Copyright The TensorFlow Authors. All Rights Reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
