# Spatial Regression Methods

## Why Spatial Regression?

Ordinary least squares (OLS) regression assumes independent residuals. When spatial autocorrelation exists, OLS produces:
- **Underestimated standard errors** → Inflated t-statistics → False significance
- **Inefficient estimates** → Suboptimal predictions
- **Biased inference** → Wrong conclusions about relationships

Spatial regression models extend OLS to account for spatial dependence, producing valid inference and better predictions when clustering/spillover effects exist.

---

## Spatial Regression Variants

### Spatial Lag Regression (SAR)

**Concept:** Response variable depends on predictor values AND on spatially-lagged response in neighboring locations.

**Model:**
y = ρWy + Xβ + ε

Where:
- y = response variable (e.g., crime rate)
- ρWy = spatial lag term (response in neighbor locations, weighted by ρ)
- X = predictor matrix
- β = predictor coefficients
- ε = error term

**Interpretation of ρ (spatial lag coefficient):**
- ρ > 0: Positive spatial spillover (e.g., crime reinforces crime in neighbors)
- ρ ≈ 0: No spillover effect (use OLS instead)
- ρ < 0: Negative spillover (rare; e.g., competition effects)
- ρ magnitude: Strength of spillover (|ρ| → 1 = strong spatial dependence)

**When to use SAR:**
- **Endogenous spillover effects:** Crime influences neighboring crime, property values influence neighbors, disease spreads to neighbors
- **Policy implications:** Changes in one location affect neighbors

**Examples:**
- **Real estate:** Property values influenced by neighbor prices (price spillover)
- **Crime:** Offenders drawn to areas with existing crime (crime concentration)
- **Retail:** Sales volume influenced by competitor density in neighboring areas
- **Epidemiology:** Disease spread through social networks (contagion effects)

**Key Parameters:**
- `weights_mode`: Neighborhood structure (queen, rook, k_nearest, distance_band)
- `row_standardize`: Normalize weights (yes, recommended)

---

### Spatial Error Regression (SEM)

**Concept:** Spatial dependence operates through residual term, not response. Captures omitted variables, measurement error, or unobserved spillovers with spatial structure.

**Model:**
y = Xβ + u
u = λWu + ε

Where:
- y = response
- X = predictors
- β = coefficients
- u = spatially-structured error
- λWu = autocorrelated error component
- ε = independent error

**Interpretation of λ (spatial error coefficient):**
- λ > 0: Positive spatial correlation in errors (nearby predictions covary)
- λ ≈ 0: No spatial correlation in errors (use OLS)
- λ < 0: Negative correlation (rare)

**When to use SEM:**
- **Exogenous spatial dependence:** Unobserved factors create spatial correlation
- **Confounding:** Omitted variables with spatial pattern

**Examples:**
- **Environmental contamination:** Omitted pollution source creates spatially-correlated residuals
- **Socioeconomic analysis:** Unmeasured neighborhood characteristics cause spatial residual correlation
- **Climate modeling:** Unobserved atmospheric patterns create spatial error structure

**Difference from SAR:**
| Aspect | SAR | SEM |
|--------|---|---|
| Mechanism | Endogenous spillover | Exogenous confounding |
| Predictions | Depend on neighbor values | Depend only on predictors |
| Interpretation | Direct causal links | Shared unmeasured factors |
| Coefficient bias | If ignored: Biased β | If ignored: Valid β, underestimated SE |

---

### Geographically Weighted Regression (GWR)

**Concept:** Local regression producing location-specific coefficients. Replaces single global βj with location-varying βj(u), revealing spatial heterogeneity in relationships.

**Model:**
y = X β(u) + ε

Where β(u) varies smoothly across space. Neighbors contribute more to local estimates than distant locations (weighted by bandwidth).

**Key Parameters:**
- **Bandwidth:** Controls geographic extent of local estimation
  - Small bandwidth: Sharp local variation, higher variance
  - Large bandwidth: Smooth global coefficients, lower variance
  - Auto-selected via AICc cross-validation

**Output:**
- Local coefficient raster for each predictor
- Local R-squared, residuals, uncertainty estimates

**When to use GWR:**
- **Spatial heterogeneity:** Predictor-response relationships differ by location
- **Market segmentation:** Different drivers in different regions
- **Environmental variation:** Habitat factors affect species differently by region

**Examples:**
- **Real estate:** Impact of property characteristics (lot size, age) varies by neighborhood
- **Health:** Pollution effects on respiratory disease vary by genetic/healthcare infrastructure
- **Ecology:** Climate factors affect species differently in core vs. marginal habitat
- **Socioeconomics:** Education's impact on income varies by regional labor markets

**Interpretation Advantages:**
- Reveals WHERE relationships break down
- Identifies market/regional segmentation
- Detects interaction effects (implicit in spatial heterogeneity)

**Computational Cost:** Higher than SAR/SEM; practical for datasets with hundreds to low thousands of features. For millions of features, use subsetting.

**Pitfalls:**
- Over-fitting with small bandwidth on sparse data
- Multicollinearity locally can inflate coefficients
- Multiple comparison issue (reporting only significant local relationships)

---

## Regression Workflow

```
1. Prepare data: Response + predictors + feature geometry
   ↓
2. RUN ORDINARY REGRESSION (baseline)
   → Examine residual map for spatial clustering
   ↓
3. TEST SPATIAL AUTOCORRELATION (Moran's I on OLS residuals)
   → Is autocorrelation significant? (Yes → continue; No → OLS sufficient)
   ↓
4. CHOOSE REGRESSION TYPE
   ├─ If endogenous spillover expected → SAR
   ├─ If exogenous confounding likely → SEM
   └─ If heterogeneity across space → GWR
   ↓
5. FIT SPATIAL MODEL
   → Examine coefficients, significance, diagnostics
   ↓
6. VALIDATE & COMPARE
   → Cross-validation RMSE vs. OLS
   → AICc comparison
   → Residual spatial autocorrelation
   ↓
7. INTERPRET & USE
   → If SAR: Report ρ + predictor effects
   → If SEM: Report β (now valid) + λ (spatial structure strength)
   → If GWR: Map local coefficients; identify heterogeneous patterns
```

---

## Diagnostic Statistics

| Statistic | Target Value | Indicates |
|-----------|---|---|
| **Moran's I on residuals** | Near 0 (non-significant p-value) | Spatial model adequacy |
| **AICc (vs. OLS)** | Lower is better | Model fit improvement |
| **RMSE (cross-validation)** | Lower is better | Prediction accuracy |
| **ρ or λ significance** | p < 0.05 | Spatial term necessary |
| **Condition number (GWR)** | < 100 preferred | Multicollinearity severity |

---

## Neighborhood Definition

**Guidance:**
- **Queen (polygon neighbor):** Standard for areal data
- **k-nearest (k=6-10):** Robust to irregular point spacing
- **Distance-band:** When explicit distance cutoff makes sense
- **Network:** For flows/movement (disease, traffic, information)

**Sensitivity:** Always test multiple neighborhood definitions to verify robustness. If conclusions change dramatically, investigate spatial structure further.

---

## Regression Troubleshooting

| Problem | Cause | Solution |
|---------|---|---|
| λ/ρ not significant | No spatial dependence | Use OLS instead; save degrees of freedom |
| Coefficients flip sign after adding spatial term | Omitted spatially-correlated variable | Add missing predictor or use GWR to capture heterogeneity |
| GWR local coefficients erratic | Small bandwidth, overfitting | Increase bandwidth; validate with cross-validation |
| High residual autocorrelation after SAR/SEM | Model misspecification | Check for omitted variables, non-linear relationships |

---

## References

- Anselin, L. (1988). *Spatial Econometrics: Methods and Models*. Kluwer Academic.
- Brunsdon, C., Fotheringham, A. S., & Charlton, M. E. (1996). "Geographically Weighted Regression." *Journal of the Royal Statistical Society Series B*, 58, 431-443.
- Fotheringham, A. S., Brunsdon, C., & Charlton, M. E. (2002). *Geographically Weighted Regression: The Analysis of Spatially Varying Relationships*. Wiley.
- Vega, S. H., & Elhorst, J. P. (2015). "The SLX Model." *Journal of Regional Science*, 55(3), 339-363.
