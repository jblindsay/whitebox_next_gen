# Spatial Autocorrelation & Clustering Analysis

## Concepts

**Spatial autocorrelation** measures whether values at nearby locations are more similar (positive autocorrelation), more different (negative autocorrelation), or randomly distributed (no autocorrelation). Positive spatial autocorrelation is ubiquitous in real data—nearby observations tend to be similar due to shared underlying processes, proximity to sources, or persistence.

**Why it matters:**
- Violates standard statistical assumptions of independence
- Indicates need for spatial modeling (kriging, spatial regression)
- Reveals clustering patterns for resource/risk management
- Tests hypothesis: "Is clustering significant or just random variation?"

---

## Autocorrelation Analysis Tools

### Global Moran's I

**Purpose:** Single summary index testing whether entire study area exhibits spatial autocorrelation.

**Interpretation:**
- **I > 0**: Positive autocorrelation (similar values cluster)
- **I < 0**: Negative autocorrelation (dissimilar values alternate)
- **I ≈ 0**: Random spatial arrangement (no autocorrelation)

**Statistical Testing:**
- **Asymptotic inference**: Assumes large-sample normality; fast computation
- **Permutation inference**: Generates null distribution via random shuffling; more robust for non-normal data

**Parameters:**
- `weights_mode`: Defines neighborhood structure
  - `queen`: Adjacent polygons sharing edge/corner
  - `rook`: Adjacent polygons sharing edge only
  - `k_nearest`: k closest neighbors (robust to boundary effects)
  - `distance_band`: All points/polygons within threshold distance
- `row_standardize`: Normalize weights to sum to 1 per row (recommended: yes)

**Output:**
- I statistic, expected I (under CSR), variance, z-score, p-value
- Permutation distribution (if requested) showing null vs. observed

**Interpretation Guide:**
- Significant positive I (p < 0.05): Strong clustering; consider spatial modeling
- Significant negative I: Rare in practice; suggests alternating values
- Non-significant: Could use standard statistical methods without spatial adjustment

**Practical Use:**
- First step in spatial analysis: "Is my data spatially dependent?"
- If significant → Use kriging, spatial regression, LISA for detailed patterns
- If not significant → Standard statistical methods may suffice

---

### Local Moran's I (LISA)

**Purpose:** Feature-level clustering analysis identifying local clusters and outliers.

**Key Insight:** While Global Moran's I summarizes the entire study area, LISA **identifies where** clusters occur and **classifies** each feature into cluster type.

**Cluster Classifications:**
- **HH** (High-High): Feature with high value surrounded by high-value neighbors (e.g., wealthy neighborhood surrounded by wealthy areas)
- **LL** (Low-Low): Feature with low value surrounded by low-value neighbors (e.g., poverty cluster)
- **HL** (High-Low): High-value outlier surrounded by low-value neighbors
- **LH** (Low-High): Low-value outlier surrounded by high-value neighbors
- **NS** (Not Significant): Feature not part of statistically significant cluster

**Parameters:** Same as Global Moran's I, plus:
- `alpha`: Significance threshold (default: 0.05)
- `fdr_correction`: Apply Benjamini-Hochberg False Discovery Rate correction (recommended: yes for many tests)

**Output:**
- Feature geometry + LISA values + cluster classification
- Optionally: p-values and significance for each feature

**Interpretation Tips:**
- Map cluster classifications to visualize spatial structure
- Focus management on HH (high hotspots) or LL (low coldspots)
- Investigate HL, LH outliers for anomalies or data quality issues

**Applications:**
- **Public health:** Identify disease clusters (LL = protected areas, HH = outbreak zones)
- **Crime analysis:** Crime hotspots (HH) and safe zones (LL)
- **Environmental justice:** Pollution concentration clusters (HH) and gaps (LL)
- **Socioeconomics:** Poverty clusters, wealth clusters, mixed neighborhoods

---

### Getis-Ord Gi / Gi*

**Purpose:** Direct hotspot/coldspot identification measuring local concentration of high/low values.

**Key Difference from Moran's I:**
- Moran's I: Tests correlation with global mean
- Gi*: Tests concentration of high/low values directly
- Result: Gi* more interpretable for practitioners (positive z = hotspot, negative z = coldspot)

**Variants:**
- **Gi** (exclude self): Neighborhood influence without the feature itself
- **Gi*** (include self): Neighborhood influence including the feature (recommended default)

**Output:**
- Z-scores for each feature (standardized concentration index)
- P-values (asymptotic or permutation-based significance)
- Classification: Hotspot (z > critical value), Coldspot (z < -critical value), Not Significant

**Critical Values for α = 0.05:**
- z > 1.96: Significant hotspot (95% confidence)
- z < -1.96: Significant coldspot (95% confidence)
- |z| < 1.96: Not significant at 95% level

**Interpretation:**
- **Large positive z**: Strong high-value concentration (e.g., retail sales hotspot)
- **Large negative z**: Strong low-value concentration (e.g., pollution gap/cold spot)
- **z near 0**: Values close to area average

**Advantages over LISA:**
- More intuitive: positive = hot, negative = cold
- Directionally unambiguous (HL/LH ambiguity resolved)
- Better for continuous gradient mapping (z-scores vs. categorical classification)

**Applications:**
- **Retail/business:** Sales hotspots identify high-performing locations
- **Emergency services:** Incident hotspots guide resource allocation
- **Environmental:** Pollution concentration zones vs. clean areas
- **Climate:** Temperature anomalies (heat waves vs. cold snaps)

---

## Neighborhood Definition & Sensitivity

All autocorrelation tools depend on **weights matrix** defining which features are neighbors. Weights definition significantly affects results.

**Guidance by Context:**

| Context | Recommended | Rationale |
|---------|---|---|
| **Polygon data** (census tracts, counties) | Queen (adjacent) | Natural polygon neighbors |
| **Point data** | k-nearest (k=8-10) | Robust to boundary effects; adaptive density |
| **Sparse irregular patterns** | Distance-band (fixed distance) | Uniform neighborhoods for all features |
| **Network data** (disease spread, traffic) | Network neighbors | Follows actual connectivity |

**Sensitivity Analysis:**
Always test multiple neighborhood definitions to verify robustness:
1. Run analysis with queen / k_nearest / distance_band
2. Compare results: Do cluster locations persist?
3. If cluster locations change dramatically: Results may be artifacts of neighborhood choice

---

## Clustering Analysis Workflow

```
1. Load feature data (e.g., disease incidence by county)
   ↓
2. GLOBAL MORAN'S I
   → Is clustering significant? (Yes → continue; No → stop)
   ↓
3. LISA CLUSTERING
   → Identify HH (hotspots), LL (coldspots), HL/LH (outliers)
   → Map classifications
   ↓
4. GETIS-ORD GI*
   → Map z-scores for gradient view
   → Compare to LISA for pattern consistency
   ↓
5. INTERPRET & ACT
   → Hotspots: Prioritize management resources
   → Outliers: Investigate for data quality or causal anomalies
   → Coldspots: Understand protective factors
```

---

## Troubleshooting

| Problem | Cause | Solution |
|---------|---|---|
| Non-significant Global I but clear visual clusters | Weak global correlation + local clustering | Use LISA; define neighborhoods carefully |
| Clusters shift with neighborhood definition | Sensitive results or boundary artifacts | Test multiple definitions; use k-nearest for robustness |
| HL/LH outliers unexpectedly common | Extreme outliers in data | Check for data entry errors; consider robust transformations |
| High significance but small effect size | Large sample inflates p-values | Report z-scores/magnitudes alongside p-values |

---

## Multiple Testing Correction

When running LISA on hundreds of features, some clusters will appear by chance alone (false positives). **FDR correction** (Benjamini-Hochberg) controls expected proportion of false discoveries:

- Without correction: Many spurious clusters
- With FDR (α = 0.05): ~5% of reported clusters expected to be false
- Conservative choice for critical applications (e.g., disease cluster verification)

---

## References

- Moran, P. A. P. (1950). "Notes on Continuous Stochastic Phenomena." *Biometrika*, 37(1-2), 17-23.
- Anselin, L. (1995). "Local Indicators of Spatial Association—LISA." *Geographical Analysis*, 27(2), 93-115.
- Getis, A., & Ord, J. K. (1992). "The Analysis of Spatial Association by Use of Distance Statistics." *Geographical Analysis*, 24(3), 189-206.
- Cliff, A. D., & Ord, J. K. (1973). *Spatial Autocorrelation*. Pion.
