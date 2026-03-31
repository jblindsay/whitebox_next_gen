# EPSG Workflows Priority Shortlist

This shortlist prioritizes missing EPSG codes from `whitebox_workflows` that are most feasible to add in near-term batches using projection families already implemented in `wbprojection`.

## Selection Criteria

- Same projection method as existing supported CRS (TM, UTM, LCC, Hotine Oblique Mercator).
- Parameter patterns that match existing helper approaches.
- Regionally useful systems likely to appear in practical workflows.

## Batch A (implemented in this pass)

- `2057` Rassadiran / Nakhl-e Taqi (Hotine Oblique Mercator)
- `2059` ED50(ED77) / UTM zone 39N
- `2060` ED50(ED77) / UTM zone 40N
- `2061` ED50(ED77) / UTM zone 41N

## Batch B (implemented in this pass)

- `2040` Locodjo 1965 / UTM zone 30N
- `2041` Abidjan 1987 / UTM zone 30N
- `2042` Locodjo 1965 / UTM zone 29N
- `2043` Abidjan 1987 / UTM zone 29N
- `2063` Dabola 1981 / UTM zone 28N
- `2064` Dabola 1981 / UTM zone 29N
- `2067` Naparima 1955 / UTM zone 20N

## Batch C (implemented in this pass)

- `2068` through `2076` ELD 1979 Libya TM zones (5 to 13)
- `2077` through `2080` ELD 1979 UTM zones (32N to 35N)
- `2085` NAD27 Cuba Norte (LCC)
- `2086` NAD27 Cuba Sur (LCC)

## Batch D (implemented in this pass)

- `2087` ELD79 / TM 12 NE
- `2088` Carthage / TM 11 NE
- `2089`, `2090` Yemen NGN96 / UTM zones 38N-39N
- `2091`, `2092` South Yemen / GK zones 8-9
- `2093` Hanoi 1972 / GK 106 NE
- `2094` WGS 72BE / TM 106 NE
- `2095` Bissau / UTM zone 28N
- `2096`, `2097`, `2098` Korean 1985 belts (East/Central/West)

## Batch E (implemented in this pass)

- `2105` through `2132` NZGD2000 local circuit TM systems
- `2133`, `2134`, `2135` NZGD2000 / UTM zones 58S-60S

## Batch F (implemented in this pass)

- `2136`, `2137` Accra grid systems
- `2138` NAD27(CGQ77) Quebec Lambert
- `2148` through `2152` NAD83(CSRS) UTM variant aliases

## Batch G (implemented in this pass)

- `2153` NAD83(CSRS) / UTM zone 11N
- `2158` IRENET95 / UTM zone 29N
- `2159`, `2160` Sierra Leone 1924 grids
- `2161`, `2162` Sierra Leone 1968 UTM zones 28N-29N
- `2164`, `2165` Locodjo/Abidjan TM 5 NW
- `2169` Luxembourg 1930 Gauss
- `2170` MGI Slovenia Grid

## Batch H (implemented in this pass)

- `2166`, `2167`, `2168` Pulkovo adjusted 3-degree GK zones 3-5
- `2397`, `2398`, `2399` matching EPSG authority aliases for same GK zones

## Batch I (implemented in this pass)

- `32201` through `32260` WGS 72 / UTM northern hemisphere zones 1N-60N
- `32301` through `32360` WGS 72 / UTM southern hemisphere zones 1S-60S

## Batch J (implemented in this pass)

- `2494` through `2758` contiguous high-volume block (261 Pulkovo 1942/1995 GK systems + 4 regional outliers)

## Batch K (implemented in this pass)

- `3580` through `3751` contiguous high-volume block (NAD83(NSRS2007) StatePlane families, NSRS2007 UTM aliases, and NAD83(HARN) UTM entries)

## Batch L (implemented in this pass)

- `2172` through `2175` Pulkovo 1942 Adj 1958 Poland zones II-V (double stereographic and TM)

## Batch M (implemented in this pass)

- `2188` through `2192` and `2195` through `2198` (Azores/Madeira UTM systems, ED50 France EuroLambert, NAD83(HARN) UTM 2S, ETRS89 Kp2000 variants)

## Batch N (implemented in this pass)

- `2205` through `2213` (NAD83 Kentucky North, ED50 3-degree GK zones 9-15, ETRS89 TM 30 NE)

## Batch O (implemented in this pass)

- `2200` through `2204`, `2214` through `2220`, and `2222` through `2226` plus `2228` (ATS77/REGVEN/NAD27/StatePlane families)

## Batch P (implemented in this pass)

- `2252` through `2262`, `2264` through `2271`, and `2274` through `2282` (NAD83 StatePlane foot/US-foot families: Michigan, Mississippi, Montana, New Mexico, New York, North Carolina, North Dakota, Oklahoma, Oregon, Pennsylvania, Tennessee, Texas, Utah)

## Batch Q (implemented in this pass)

- `2287` through `2292`, `2294`, `2295`, and `2308` through `2313` (NAD83 Wisconsin StatePlane ftUS, ATS77/NAD83(CSRS) Prince Edward Island and Nova Scotia systems, Batavia/WGS84/Garoua/Kousseri TM families)

## Batch R (implemented in this pass)

- `2315` through `2325` and `2327` through `2333` (Campo Inchauspe UTM, PSAD56/Ain el Abd LCC systems, ED50 TM27-45, Xian 1980 GK zones 13-19)

## Batch S (implemented in this pass)

- `2334` through `2390` contiguous Xian 1980 GK families (6-degree zones/CM and 3-degree zones/CM)

## Batch T (implemented in this pass)

- `2314` Trinidad 1903 / Trinidad Grid (Cassini-Soldner)

## Batch U (implemented in this pass)

- `4001` through `4016`, `4018` through `4029`, `4031` through `4038`, `4044` through `4063` (legacy workflows parity block combining ellipsoid/geographic definitions with Moldova and RGRDC TM/UTM projected systems)

## Batch V (implemented in this pass)

- Step 2 first: `2391` through `2396`, `2400` through `2442`, `2867` through `2888`, `2891` through `2954`.
- Step 1 second: `4120` through `4147`, `4149` through `4151`, `4153` through `4166`, `4168` through `4176`, `4178` through `4185`.

## Notes

- This shortlist is feasibility-prioritized, not exhaustive.
- Some missing codes depend on uncommon datums, prime meridians, or unit systems and may need dedicated datum definitions for high-fidelity behavior.

## Usage-Priority Ordering (Step 3)

This is a practical rollout order biased toward broad GIS workflow relevance and shared projection families.

1. Continue with remaining missing codes by contiguous family blocks from the parity report to maximize test and maintenance efficiency.

## Next Candidate Blocks (post-2026-03-15 parity refresh)

These are the best near-term targets from current missing contiguous ranges, balancing impact and implementation risk.

1. `4001-4016`, `4018-4029`, `4031-4038`, `4044-4063`.
2. `4120-4147`, `4149-4151`, `4153-4166`, `4168-4176`, `4178-4185`.
3. `4190-4229`, `4231-4257`, `4270-4282`, `4291-4304`, `4306-4319`.
4. `2391-2442` and `2867-2954` (strong contiguous volume, mostly projection-family style additions).

These blocks are expected to yield better parity gains than isolated one-off codes and preserve the existing family-oriented maintenance approach in src/epsg.rs.
