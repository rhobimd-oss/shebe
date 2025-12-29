# Grep Ground Truth for Eigen C++ Symbol Reference Tests

**Document:** 015-grep-ground-truth-02.md
**Generated:** 2025-12-28 20:37 UTC
**Repository:** ~/gitlab/libeigen/eigen
**Related:** 015-shebe-cpp-accuracy-test-plan-01.md

## Summary Table

| Symbol | Lines Matching | Files Matching | Category |
|--------|----------------|----------------|----------|
| `MatrixXd` | 464 | 125 | A |
| `CwiseBinaryOp` | 240 | 44 | A |
| `PlainObjectBase` | 70 | 15 | A |
| `EIGEN_DEVICE_FUNC` | 6230 | 246 | A |
| `Vector3d` | 49 | 31 | C |
| `DenseBase` | 349 | 53 | C |
| `traits` | 4944 | 331 | B |
| `Index` | 16667 | 570 | B |
| `Scalar` | 16894 | 590 | B |
| `Dynamic` | 1857 | 375 | B |

## Category Analysis

### Category A: Distinct Symbols (Low Ambiguity)

These symbols have unique names unlikely to cause false positives.

| Symbol              | Lines | Files | Lines/File | Notes                                 |
|---------------------|-------|-------|------------|---------------------------------------|
| `MatrixXd`          | 464   | 125   | 3.7        | Moderate usage, well-distributed      |
| `CwiseBinaryOp`     | 240   | 44    | 5.5        | Concentrated in core/expression files |
| `PlainObjectBase`   | 70    | 15    | 4.7        | Few files, CRTP base class            |
| `EIGEN_DEVICE_FUNC` | 6230  | 246   | 25.3       | Very high frequency macro             |

### Category B: Generic Symbols (High Ambiguity)

These symbols have common names likely to match unrelated code.

| Symbol    | Lines | Files | Lines/File | Notes                            |
|-----------|-------|-------|------------|----------------------------------|
| `traits`  | 4944  | 331   | 14.9       | Extremely generic, many contexts |
| `Index`   | 16667 | 570   | 29.2       | Ubiquitous, highest line count   |
| `Scalar`  | 16894 | 590   | 28.6       | Ubiquitous, most files           |
| `Dynamic` | 1857  | 375   | 5.0        | Common but less dense            |

### Category C: Hierarchical Symbols

Symbols that participate in type hierarchies.

| Symbol      | Lines | Files | Lines/File | Notes                         |
|-------------|-------|-------|------------|-------------------------------|
| `Vector3d`  | 49    | 31    | 1.6        | Sparse usage, mostly examples |
| `DenseBase` | 53    | 53    | 6.6        | Base class, moderate usage    |

## Detailed Results by Symbol

### MatrixXd (Category A)

**Top 10 files by match count:**
```
     37 demos/mix_eigen_and_c/binary_library.cpp
     29 demos/mix_eigen_and_c/binary_library.h
     24 unsupported/test/NNLS.cpp
     22 unsupported/test/NonLinearOptimization.cpp
     19 unsupported/test/levenberg_marquardt.cpp
     16 test/evaluators.cpp
     16 test/eigensolver_generic.cpp
      9 unsupported/test/kronecker_product.cpp
      9 test/ref.cpp
      9 test/bdcsvd.cpp
```

### CwiseBinaryOp (Category A)

**Top 10 files by match count:**
```
     43 Eigen/src/SparseCore/SparseCwiseBinaryOp.h
     33 unsupported/Eigen/CXX11/src/Tensor/TensorBase.h
     14 unsupported/Eigen/CXX11/src/Tensor/TensorExpr.h
     13 Eigen/src/Core/functors/BinaryFunctors.h
     13 Eigen/src/Core/CwiseBinaryOp.h
     12 unsupported/Eigen/src/SpecialFunctions/SpecialFunctionsArrayAPI.h
     12 unsupported/Eigen/src/AutoDiff/AutoDiffVector.h
     11 Eigen/src/Core/CoreEvaluators.h
      8 Eigen/src/Core/ProductEvaluators.h
      6 unsupported/Eigen/src/AutoDiff/AutoDiffScalar.h
```

### PlainObjectBase (Category A)

**Top 10 files by match count:**
```
     23 Eigen/src/Core/PlainObjectBase.h
     12 Eigen/src/Core/CwiseNullaryOp.h
      7 Eigen/src/Core/CoreEvaluators.h
      4 Eigen/src/Core/Ref.h
      4 Eigen/src/Core/Random.h
      4 Eigen/src/Core/Matrix.h
      4 Eigen/src/Core/MapBase.h
      3 Eigen/src/Core/DenseBase.h
      2 Eigen/src/Core/Map.h
      2 Eigen/src/Core/Array.h
```

### EIGEN_DEVICE_FUNC (Category A)

**Top 10 files by match count:**
```
    233 Eigen/src/Core/MathFunctions.h
    218 Eigen/src/Core/CoreEvaluators.h
    186 Eigen/src/Core/arch/GPU/PacketMath.h
    177 unsupported/Eigen/CXX11/src/Tensor/TensorBase.h
    173 Eigen/src/Core/GenericPacketMath.h
    160 Eigen/src/Core/DenseStorage.h
    138 Eigen/src/Core/arch/Default/Half.h
    117 Eigen/src/Core/functors/UnaryFunctors.h
    116 Eigen/src/Geometry/Transform.h
    108 Eigen/src/Core/arch/Default/BFloat16.h
```

### Vector3d (Category C)

**Top 10 files by match count:**
```
      6 unsupported/test/openglsupport.cpp
      5 doc/snippets/HouseholderSequence_HouseholderSequence.cpp
      3 test/ref.cpp
      2 unsupported/test/splines.cpp
      2 unsupported/test/autodiff.cpp
      2 test/nullary.cpp
      2 doc/snippets/MatrixBase_isOrthogonal.cpp
      2 doc/examples/tut_arithmetic_dot_cross.cpp
      2 doc/examples/tut_arithmetic_add_sub.cpp
      2 doc/examples/QuickStart_example2_fixed.cpp
```

### DenseBase (Category C)

**Top 10 files by match count:**
```
     73 Eigen/src/Core/CwiseNullaryOp.h
     35 Eigen/src/Core/VectorwiseOp.h
     32 Eigen/src/Core/DenseBase.h
     22 Eigen/src/Core/Select.h
     16 Eigen/src/Core/FindCoeff.h
     15 Eigen/src/Core/PlainObjectBase.h
     13 Eigen/src/Core/Visitor.h
     12 Eigen/src/Core/Random.h
      8 Eigen/src/Core/util/IndexedViewHelper.h
      7 Eigen/src/Core/Redux.h
```

### traits (Category B - High Ambiguity)

**Top 10 files by match count:**
```
    427 Eigen/src/Core/products/GeneralBlockPanelKernel.h
    412 Eigen/src/Core/arch/RVV10/PacketMath.h
    348 Eigen/src/Core/arch/RVV10/PacketMath2.h
    338 Eigen/src/Core/arch/RVV10/PacketMath4.h
    191 Eigen/src/Core/arch/RVV10/PacketMathFP16.h
    121 Eigen/src/Core/functors/UnaryFunctors.h
    103 Eigen/src/Core/GenericPacketMath.h
     97 Eigen/src/Core/CoreEvaluators.h
     82 Eigen/src/Core/arch/NEON/TypeCasting.h
     80 Eigen/src/Core/arch/Default/GenericPacketMathFunctions.h
```

### Index (Category B - High Ambiguity)

**Top 10 files by match count:**
```
    592 Eigen/src/Core/arch/AltiVec/MatrixProduct.h
    468 Eigen/src/SparseCore/SparseMatrix.h
    323 Eigen/src/Core/CoreEvaluators.h
    293 unsupported/Eigen/CXX11/src/Tensor/TensorContractionSycl.h
    229 unsupported/test/cxx11_tensor_image_patch_sycl.cpp
    219 unsupported/test/cxx11_tensor_contract_sycl.cpp
    210 Eigen/src/OrderingMethods/Eigen_Colamd.h
    208 Eigen/src/Core/products/GeneralBlockPanelKernel.h
    204 unsupported/Eigen/CXX11/src/Tensor/TensorIndexList.h
    198 unsupported/test/cxx11_tensor_reduction_sycl.cpp
```

### Scalar (Category B - High Ambiguity)

**Top 10 files by match count:**
```
    494 test/packetmath.cpp
    373 Eigen/src/Core/MathFunctions.h
    365 Eigen/src/Core/functors/UnaryFunctors.h
    294 unsupported/Eigen/src/SpecialFunctions/SpecialFunctionsImpl.h
    292 test/array_cwise.cpp
    233 Eigen/src/Core/products/GeneralBlockPanelKernel.h
    221 Eigen/src/Core/arch/Default/GenericPacketMathFunctions.h
    211 unsupported/Eigen/CXX11/src/Tensor/TensorBase.h
    205 blas/level3_impl.h
    204 Eigen/src/Core/functors/BinaryFunctors.h
```

### Dynamic (Category B - High Ambiguity)

**Top 10 files by match count:**
```
     69 test/dense_storage.cpp
     40 unsupported/Eigen/src/SparseExtra/BlockSparseMatrix.h
     27 test/block.cpp
     25 Eigen/src/Core/PlainObjectBase.h
     25 Eigen/src/Core/Matrix.h
     24 unsupported/Eigen/src/Eigenvalues/ArpackSelfAdjointEigenSolver.h
     23 unsupported/test/sparse_extra.cpp
     22 test/indexed_view.cpp
     21 Eigen/src/Core/util/IntegralConstant.h
     20 unsupported/Eigen/CXX11/src/TensorSymmetry/DynamicSymmetry.h
```

## Observations

### Test Plan Predictions vs Actual

| Symbol              | Test Plan Predicted Files | Actual Files | Delta |
|---------------------|---------------------------|--------------|-------|
| `MatrixXd`          | 125   | 125 | 0    |
| `CwiseBinaryOp`     | 44    | 44  | 0    |
| `PlainObjectBase`   | 15    | 15  | 0    |
| `EIGEN_DEVICE_FUNC` | 246   | 246 | 0    |
| `Vector3d`          | 31    | 31  | 0    |
| `DenseBase`         | 53    | 53  | 0    |
| `traits`            | 214   | 331 | +117 |
| `Index`             | 499   | 570 | +71  |
| `Scalar`            | 540   | 590 | +50  |
| `Dynamic`           | 375   | 375 | 0    |

The test plan predictions for generic symbols (Category B) underestimated actual file counts,
which is expected given the high ambiguity of these symbols.

## Next Steps

This ground truth will be compared against:
1. Shebe `find_references` results (015-shebe-cpp-accuracy-results-03.md)
2. Serena `find_referencing_symbols` results (015-serena-cpp-accuracy-results-04.md)
3. Final comparison analysis (015-cpp-accuracy-comparison-05.md)
