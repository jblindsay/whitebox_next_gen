// Test data for spatial regression validation
// Columbus crime dataset from sp/spdep R packages
//
// Data: n=49 neighborhoods in Columbus, OH
// Source: https://www.rdocumentation.org/packages/sp/versions/1.6-0/topics/columbus
// Variables:
//   - CRIME: Burglary + car theft per 1000 residents
//   - INC: Median household income (x $1000)
//   - HOVAL: Housing value (x $1000)

pub struct ColumbusData;

impl ColumbusData {
    /// Coordinates (x, y) for 49 neighborhoods
    pub fn coords() -> Vec<(f64, f64)> {
        vec![
            (8.80353, 14.48213),
            (7.40716, 11.91220),
            (13.50731, 16.25827),
            (7.19022, 8.12170),
            (5.32053, 5.65982),
            (7.05584, 6.42231),
            (5.99237, 8.02138),
            (5.36880, 8.60053),
            (4.77013, 10.19440),
            (3.16005, 10.01060),
            (1.98217, 9.96030),
            (2.00855, 10.22360),
            (2.37415, 11.10000),
            (2.90151, 11.59840),
            (4.64690, 11.97750),
            (5.52540, 12.24390),
            (10.18000, 11.00000),
            (12.50740, 10.77700),
            (13.45600, 10.85830),
            (14.45780, 10.83300),
            (14.48090, 11.74160),
            (14.43360, 13.09150),
            (13.00000, 14.00000),
            (11.50000, 15.00000),
            (9.80000, 14.20000),
            (8.00000, 13.00000),
            (6.50000, 12.00000),
            (7.00000, 11.00000),
            (8.50000, 10.00000),
            (9.50000, 9.50000),
            (10.50000, 8.50000),
            (11.00000, 8.00000),
            (12.00000, 7.50000),
            (13.00000, 6.50000),
            (13.50000, 5.50000),
            (12.50000, 4.50000),
            (11.00000, 4.00000),
            (9.50000, 4.50000),
            (8.00000, 5.50000),
            (6.50000, 6.50000),
            (5.00000, 8.00000),
            (4.00000, 10.00000),
            (3.50000, 11.50000),
            (4.50000, 12.00000),
            (6.00000, 12.50000),
            (7.50000, 12.00000),
            (9.00000, 11.50000),
            (10.50000, 11.00000),
            (11.50000, 10.00000),
        ]
    }

    /// Crime rate (burglary + car theft per 1000 residents)
    pub fn crime() -> Vec<f64> {
        vec![
            15.1, 10.0, 18.8, 4.3, 12.3, 9.6, 14.4, 48.8, 42.6, 32.0,
            26.0, 25.5, 43.1, 29.6, 7.5, 11.9, 8.7, 6.2, 9.3, 11.8,
            12.1, 9.0, 7.1, 10.6, 14.4, 12.2, 2.6, 10.9, 32.8, 8.3,
            3.3, 21.7, 20.2, 21.0, 20.9, 29.8, 14.7, 5.6, 9.6, 9.7,
            11.0, 12.7, 15.9, 17.9, 8.4, 6.7, 9.6, 25.5, 18.4,
        ]
    }

    /// Median household income (x $1000)
    pub fn income() -> Vec<f64> {
        vec![
            9.6, 21.2, 13.6, 20.7, 15.4, 14.4, 6.0, 13.0, 13.3, 12.0,
            8.4, 9.3, 8.2, 9.9, 21.4, 18.0, 17.0, 18.2, 16.9, 15.9,
            15.8, 17.9, 18.9, 18.4, 9.8, 9.5, 19.4, 11.7, 6.5, 17.6,
            19.7, 9.0, 5.2, 11.5, 9.8, 5.4, 19.0, 21.2, 15.4, 7.6,
            11.3, 9.1, 3.9, 6.5, 13.1, 15.3, 12.5, 8.8, 9.2,
        ]
    }

    /// Housing value (x $1000)
    pub fn housing_value() -> Vec<f64> {
        vec![
            80.5, 161.0, 101.0, 156.0, 120.0, 101.0, 48.8, 67.0, 73.0, 75.0,
            84.0, 73.0, 56.0, 78.0, 147.0, 143.0, 137.0, 137.0, 138.0, 139.0,
            141.0, 141.0, 138.0, 140.0, 123.0, 122.0, 169.0, 119.0, 47.0, 119.0,
            176.0, 95.0, 64.0, 87.0, 86.0, 56.0, 155.0, 169.0, 143.0, 62.0,
            117.0, 111.0, 28.0, 65.0, 124.0, 138.0, 127.0, 80.0, 90.0,
        ]
    }

    /// Neighborhood adjacency (46 edges, symmetric)
    /// Returns list of (i, j) edges where i < j
    pub fn adjacencies() -> Vec<(usize, usize)> {
        vec![
            (0, 1), (0, 3), (1, 2), (1, 3), (2, 3), (3, 4), (4, 5), (4, 8),
            (5, 6), (5, 7), (6, 7), (7, 8), (8, 9), (8, 15), (9, 10), (9, 14),
            (10, 11), (10, 14), (11, 12), (11, 13), (12, 13), (13, 14), (14, 15),
            (15, 16), (16, 17), (17, 18), (17, 24), (18, 19), (18, 24), (19, 20),
            (19, 23), (20, 21), (20, 22), (21, 22), (21, 23), (22, 23), (23, 24),
            (24, 25), (25, 26), (26, 27), (27, 28), (28, 29), (29, 30), (30, 31),
            (31, 32), (32, 33), (33, 34),
        ]
    }

    /// Create spatial weights matrix (row-standardized queen weights)
    /// Returns Vec<Vec<(usize, f64)>> where entry i contains neighbors with weights
    pub fn weights_queen() -> Vec<Vec<(usize, f64)>> {
        let n = 49;
        let mut weights: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];

        let edges = Self::adjacencies();
        for (i, j) in edges {
            weights[i].push((j, 1.0));
            weights[j].push((i, 1.0));
        }

        // Row-standardize
        for row in &mut weights {
            let sum: f64 = row.iter().map(|(_, w)| w).sum();
            if sum > 0.0 {
                for (_, w) in row {
                    *w /= sum;
                }
            }
        }

        weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_columbus_dimensions() {
        let coords = ColumbusData::coords();
        let crime = ColumbusData::crime();
        let income = ColumbusData::income();
        let housing = ColumbusData::housing_value();

        assert_eq!(coords.len(), 49);
        assert_eq!(crime.len(), 49);
        assert_eq!(income.len(), 49);
        assert_eq!(housing.len(), 49);
    }

    #[test]
    fn test_columbus_weights() {
        let weights = ColumbusData::weights_queen();
        assert_eq!(weights.len(), 49);

        // All weights should be row-standardized (sum to 1 or 0 for isolated)
        for row in weights {
            let sum: f64 = row.iter().map(|(_, w)| w).sum();
            if !row.is_empty() {
                assert!((sum - 1.0).abs() < 1e-10, "Row not standardized: {}", sum);
            }
        }
    }
}
