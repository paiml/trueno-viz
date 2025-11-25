# SIMD Acceleration

Trueno-viz uses SIMD (Single Instruction, Multiple Data) to accelerate
rendering operations through the trueno core library.

## Automatic Dispatch

SIMD acceleration is automatic based on CPU capabilities:

```rust
use trueno_viz::accel;

// Query available SIMD support
let info = accel::cpu_info();
println!("SSE2: {}", info.has_sse2());
println!("AVX2: {}", info.has_avx2());
println!("AVX512: {}", info.has_avx512());
println!("NEON: {}", info.has_neon());  // ARM
```

## Supported Operations

| Operation | SIMD Benefit |
|-----------|--------------|
| Color blending | 4-8x faster |
| Pixel filling | 4-16x faster |
| Statistics (min/max/sum) | 4-8x faster |
| Scale transforms | 4-8x faster |
| Line clipping | 2-4x faster |

## Explicit SIMD Control

Force specific SIMD level:

```rust
use trueno_viz::accel::{SimdLevel, set_simd_level};

// Force AVX2 (disable AVX512)
set_simd_level(SimdLevel::Avx2);

// Force scalar (disable all SIMD)
set_simd_level(SimdLevel::Scalar);

// Auto-detect (default)
set_simd_level(SimdLevel::Auto);
```

## SIMD-Accelerated Functions

### Color Operations

```rust
use trueno_viz::color::Rgba;
use trueno_viz::accel::simd_ops;

// Batch alpha blending (8 colors at once on AVX2)
let src_colors = [Rgba::RED; 8];
let dst_colors = [Rgba::BLUE; 8];
let result = simd_ops::blend_colors_batch(&src_colors, &dst_colors);
```

### Statistics

```rust
use trueno_viz::accel::simd_ops;

let data: Vec<f32> = (0..1000).map(|i| i as f32).collect();

// SIMD min/max (4-8x faster)
let (min, max) = simd_ops::minmax_f32(&data);

// SIMD sum (4x faster)
let sum = simd_ops::sum_f32(&data);
```

### Scale Transforms

```rust
use trueno_viz::scale::LinearScale;
use trueno_viz::accel::simd_ops;

let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 800.0);
let values: Vec<f32> = (0..1000).map(|i| i as f32 * 0.1).collect();

// SIMD batch transform
let pixels = simd_ops::transform_batch(&scale, &values);
```

## Benchmark Example

```rust
use trueno_viz::plots::Heatmap;
use std::time::Instant;

fn main() {
    // 1M cell heatmap
    let data: Vec<f32> = (0..1_000_000).map(|i| i as f32).collect();

    // With SIMD (default)
    let start = Instant::now();
    let heatmap = Heatmap::new(&data, 1000, 1000).build();
    heatmap.render_to_file("heatmap_simd.png").unwrap();
    println!("SIMD: {:?}", start.elapsed());

    // Without SIMD
    accel::set_simd_level(SimdLevel::Scalar);
    let start = Instant::now();
    let heatmap = Heatmap::new(&data, 1000, 1000).build();
    heatmap.render_to_file("heatmap_scalar.png").unwrap();
    println!("Scalar: {:?}", start.elapsed());
}
```

Typical results:
```text
SIMD:   125ms
Scalar: 850ms
```

## Platform-Specific Notes

### x86_64

- SSE2: Always available (baseline)
- AVX2: Haswell and newer (2013+)
- AVX512: Skylake-X and newer

### ARM64 (Apple Silicon, Raspberry Pi 4+)

- NEON: Always available
- 128-bit vectors

### WebAssembly

- SIMD128: Supported in modern browsers
- Auto-detected at runtime

## Next Chapter

Continue to [GPU Compute](./gpu.md) for even more acceleration.
