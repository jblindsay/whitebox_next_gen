//! EPSG coordinate reference system registry.
//!
//! Provides [`Crs::from_epsg`] to instantiate a projection from a numeric
//! EPSG code. The registry is compiled into the binary as a static lookup
//! table — no external database or network access is required.
//!
//! ## Coverage
//!
//! The built-in registry covers the most widely used EPSG codes:
//!
//! - **Geographic 2D** – EPSG:4326 (WGS84), 4269 (NAD83), 4267 (NAD27), 4258 (ETRS89), 4230 (ED50), 4617 (NAD83(CSRS))
//!   plus 4283 (GDA94), 4148 (Hartebeesthoek94), 4152 (NAD83(HARN)), 4167 (NZGD2000), 4189 (RGAF09), 4619 (SIRGAS95),
//!   4681 (REGVEN), 4483 (Mexico ITRF92), 4624 (RGFG95), 4284 (Pulkovo 1942), 4322 (WGS 72), 6318 (NAD83(2011)), 4615 (REGCAN95)
//! - **Web Mercator** – EPSG:3857
//! - **UTM WGS84 Northern** – EPSG:32601–32660
//! - **UTM WGS84 Southern** – EPSG:32701–32760
//! - **UTM WGS72 Northern** – EPSG:32201–32260
//! - **UTM WGS72 Southern** – EPSG:32301–32360
//! - **UTM WGS72BE Northern** – EPSG:32401–32460
//! - **UTM WGS72BE Southern** – EPSG:32501–32560
//! - **Pulkovo 1942 / 1995 GK families** – EPSG:2494–2758 (with outlier regional systems)
//! - **UTM NAD83** – EPSG:26901–26923 (zones 1–23 N)
//! - **UTM NAD83(2011)** – EPSG:6328–6348 (zones 59, 60, 1–19 N)
//! - **UTM NAD27** – EPSG:26701–26722 (zones 1–22 N)
//! - **UTM ETRS89** – EPSG:25801–25860 (zones 1–60 N)
//! - **UTM ED50** – EPSG:23001–23060 (zones 1–60 N)
//! - **NAD83(CSRS) / UTM** – EPSG:2955–2962, 3154–3160, 3761, 9709, 9713 (zones 7–24 N; active set)
//! - **NAD83(CSRS) realizations / UTM** – EPSG:22207–22222 (v2), 22307–22322 (v3), 22407–22422 (v4),
//!   22607–22622 (v6), 22707–22722 (v7), 22807–22822 (v8)
//! - **US State Plane (NAD83, meters)** – selected zones
//! - **UK National Grid** – EPSG:27700
//! - **German Gauss-Krüger** – EPSG:31466–31469
//! - **RD New (Netherlands)** – EPSG:28992
//! - **ETRS89 LCC Europe** – EPSG:3034
//! - **ETRS89 LAEA Europe** – EPSG:3035
//! - **Australian GDA94 / MGA** – EPSG:28349–28356
//! - **Australian GDA2020 / MGA** – EPSG:7849–7856 (zones 49–56)
//! - **World Mercator** – EPSG:3395
//! - **Plate Carrée** – EPSG:32662
//! - **Sweden SWEREF99 local TM** – EPSG:3007–3014
//! - **Poland CS2000 / CS92** – EPSG:2176–2180
//! - **Greece** – EPSG:2100 (GGRS87 / Greek Grid)
//! - **Hungary** – EPSG:23700 (HD72 / EOV)
//! - **Romania** – EPSG:31700 (Stereo 70)
//! - **Portugal** – EPSG:3763 (ETRS89 / TM06)
//! - **Croatia** – EPSG:3765 (HTRS96 / TM)
//! - **Estonia** – EPSG:3301 (L-EST97)
//! - **Germany** – EPSG:5243 (ETRS89 / LCC North)
//! - **Israel** – EPSG:2039 (Israeli TM Grid)
//! - **Singapore** – EPSG:3414 (SVY21 / TM)
//! - **Hong Kong** – EPSG:2326 (HK 1980 Grid)
//! - **Canada** – EPSG:3347 (Statistics Canada Lambert), 3978 (Atlas Lambert)
//! - **North America** – EPSG:3174 (Great Lakes Albers), 6350 (NAD83(2011) / CONUS Albers)
//! - **Australia** – EPSG:3111 (GDA94 / VicGrid), 3308 (GDA94 / NSW Lambert)
//! - **Additional national systems** – EPSG:3005, 3015, 3112, 3767, 3812, 3825, 3826,
//!   5179, 5181, 5182, 5186, 5187, 31256, 31257, 31258, 31287, 2046, 2047, and 7846–7848

use crate::crs::Crs;
use crate::compound_crs::CompoundCrs;
use crate::datum::{Datum, DatumTransform};
use crate::ellipsoid::Ellipsoid;
use crate::error::{ProjectionError, Result};
use crate::projections::{ProjectionKind, ProjectionParams};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

include!("legacy_parity_wkt.rs");
include!("epsg_generated_wkt.rs");
include!("epsg_generated_info.rs");

const GENERATED_BATCH1_CODES: &[u32] = &[
    2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027, 2028, 2029, 2030,
    2031, 2032, 2033, 2034, 2035, 2044, 2045, 2084, 2462, 2964, 2969, 2970, 2971, 2972,
    2973, 2975, 2976, 2977, 2978, 2980, 2981, 2987, 2988, 2992, 2994, 2995, 2996, 2997,
    2998, 2999, 3033, 3036, 3037, 3040, 3041, 3042, 3043, 3044, 3045, 3046, 3047, 3048,
    3049, 3052, 3053, 3054, 3055, 3056, 3057, 3059, 3060, 3061, 3062, 3063, 3064, 3065,
    3066, 3069, 3070, 3071, 3082, 3083, 3084, 3085, 3086, 3087, 3092, 3093, 3094, 3095,
    3096, 3097, 3098, 3099, 3100, 3101, 3102, 3106, 3107, 3109, 3141, 3142, 3144, 3145,
    3148, 3149,
];

const GENERATED_BATCH2_CODES: &[u32] = &[
    3153, 3161, 3162, 3163, 3164, 3165, 3166, 3169, 3170, 3171, 3172, 3173, 3175, 3176,
    3177, 3178, 3179, 3180, 3181, 3182, 3183, 3184, 3185, 3186, 3187, 3188, 3189, 3190,
    3191, 3192, 3193, 3194, 3195, 3196, 3197, 3198, 3199, 3201, 3202, 3203, 3296, 3297,
    3298, 3299, 3302, 3303, 3304, 3305, 3306, 3307, 3309, 3310, 3311, 3312, 3313, 3316,
    3317, 3318, 3319, 3320, 3321, 3322, 3323, 3324, 3325, 3326, 3327, 3336, 3339, 3340,
    3341, 3342, 3343, 3344, 3345, 3346, 3348, 3353, 3354, 3367, 3368, 3369, 3370, 3371,
    3372, 3373, 3374, 3389, 3390, 3391, 3392, 3393, 3396, 3397, 3398, 3399, 3415, 3416,
    3439, 3440,
];

const GENERATED_BATCH3_CODES: &[u32] = &[
    3447, 3449, 3450, 3461, 3462, 3762, 3768, 3769, 3771, 3772, 3773, 3775, 3776, 3777,
    3779, 3780, 3781, 3783, 3784, 3788, 3789, 3790, 3791, 3793, 3797, 3798, 3799, 3800,
    3801, 3802, 3814, 3815, 3816, 3829, 3890, 3891, 3892, 3920, 3968, 3969, 3970, 3979,
    4071, 4082, 4083, 4415, 4462, 4467, 4471, 4484, 4485, 4486, 4487, 4488, 4489, 4559,
    4647, 5014, 5015, 5016, 5069, 5071, 5072, 5130, 5223, 5269, 5270, 5271, 5272, 5273,
    5274, 5275, 5292, 5293, 5294, 5295, 5296, 5297, 5298, 5299, 5300, 5301, 5302, 5303,
    5304, 5305, 5306, 5307, 5308, 5309, 5310, 5311, 5316, 5320, 5321, 5325, 5337, 5355,
    5356, 5357,
];

const GENERATED_BATCH4_CODES: &[u32] = &[
    7374, 7375, 7376, 7791, 7792, 7793, 7799, 7800, 7803, 7805, 7857, 7858, 7859, 7878,
    7883, 7991, 7992, 8035, 8036, 8058, 8059, 8082, 8083, 8088, 8395, 8455, 8456, 8682,
    8687, 8692, 8693, 8826, 8836, 8837, 8838, 8839, 8840, 8903, 8909, 8910, 8950, 8951,
    9149, 9150, 9154, 9155, 9156, 9157, 9158, 9159, 9191, 9205, 9206, 9207, 9208, 9209,
    9210, 9211, 9212, 9213, 9214, 9215, 9216, 9217, 9218, 9221, 9222, 9265, 9295, 9296,
    9297, 9356, 9357, 9358, 9359, 9360, 9391, 9404, 9405, 9406, 9407, 9473, 9476, 9477,
    9478, 9479, 9480, 9481, 9482, 9487, 9488, 9489, 9490, 9491, 9492, 9493, 9494, 9674,
    9678, 9680,
];

const GENERATED_BATCH5_CODES: &[u32] = &[
    21028, 21029, 21030, 21031, 21032, 21035, 21036, 21037, 21095, 21096, 21097, 21148,
    21149, 21150, 21413, 21414, 21415, 21416, 21417, 21418, 21419, 21420, 21421, 21422,
    21423, 21453, 21454, 21455, 21456, 21457, 21458, 21459, 21460, 21461, 21462, 21463,
    21500, 21818, 22032, 22033, 22091, 22092, 22229, 22230, 22231, 22232, 22234, 22235,
    22262, 22263, 22264, 22265, 22332, 22337, 22338, 22348, 22349, 22350, 22351, 22352,
    22353, 22354, 22355, 22356, 22357, 22462, 22463, 22464, 22465, 22521, 22522, 22523,
    22524, 22525, 22641, 22642, 22643, 22644, 22645, 22646, 22648, 22649, 22650, 22651,
    22652, 22653, 22654, 22655, 22656, 22657, 22762, 22763, 22764, 22765, 22770, 23028,
    23029, 23030, 23031, 23032,
];

const GENERATED_BATCH6_CODES: &[u32] = &[
    5361, 5362, 5382, 5383, 5387, 5389, 5460, 5469, 5490, 5520, 5523, 5531, 5533, 5534,
    5535, 5536, 5537, 5538, 5539, 5562, 5563, 5564, 5565, 5566, 5567, 5568, 5569, 5596,
    5627, 5629, 5644, 5649, 5650, 5651, 5652, 5653, 5659, 5666, 5667, 5668, 5669, 5676,
    5677, 5678, 5679, 5680, 5682, 5683, 5684, 5685, 5700, 5836, 5837, 5839, 5842, 5844,
    5858, 5875, 5876, 5877, 5879, 5896, 5897, 5898, 5899, 6312, 6366, 6367, 6368, 6369,
    6370, 6371, 6381, 6382, 6383, 6384, 6385, 6386, 6387, 6622, 6623, 6624, 6634, 6635,
    6636, 6703, 6868, 7005, 7006, 7007, 7074, 7075, 7076, 7077, 7078, 7079, 7080, 7081,
    9697, 9698,
];

const GENERATED_BATCH7_CODES: &[u32] = &[
    9699, 9712, 9716, 9793, 9794, 23239, 10285, 10286, 10287, 10288, 10289, 10290,
    10291, 10306, 10314, 10315, 10316, 10317, 10329, 10477, 10665, 10674, 10731, 10732,
    10733, 20042, 20048, 20049, 20135, 20136, 20137, 20138, 20436, 20437, 20438, 20439,
    20440, 20538, 20539, 20822, 20823, 20824, 20904, 20905, 20906, 20907, 20908, 20909,
    20910, 20911, 20912, 20913, 20914, 20915, 20916, 20917, 20918, 20919, 20920, 20921,
    20922, 20923, 20924, 20925, 20926, 20927, 20928, 20929, 20930, 20931, 20932, 20934,
    20935, 20936, 21004, 21005, 21006, 21007, 21008, 21009, 21010, 21011, 21012, 21013,
    21014, 21015, 21016, 21017, 21018, 21019, 21020, 21021, 21022, 21023, 21024, 21025,
    21026, 21027, 23090, 23095,
];

const GENERATED_BATCH8_CODES: &[u32] = &[
    27573, 23240, 23830, 23831, 23832, 23833, 23834, 23835, 23836, 23837, 23838, 23839,
    23840, 23841, 23842, 23843, 23844, 23845, 23846, 23847, 23848, 23849, 23850, 23851,
    23852, 23866, 23867, 23868, 23869, 23870, 23871, 23872, 23877, 23878, 23879, 23880,
    23881, 23882, 23883, 23884, 23887, 23888, 23889, 23890, 23891, 23892, 23893, 23894,
    23946, 23947, 23948, 24047, 24048, 24305, 24306, 24311, 24312, 24313, 24342, 24343,
    24344, 24345, 24346, 24347, 24547, 24548, 24600, 24718, 24719, 24720, 25231, 25884,
    25932, 26237, 26331, 26332, 26632, 26692, 26891, 26892, 26893, 26894, 26895, 26896,
    26897, 26898, 26899, 27039, 27040, 27120, 27258, 27259, 27260, 27429, 27561, 27562,
    27563, 27564, 27571, 27572,
];

const GENERATED_BATCH9_CODES: &[u32] = &[
    2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2009, 2010, 2011, 2012, 2013, 2014,
    2015, 2016, 2062, 2066, 2081, 2082, 2083, 2099, 2101, 2102, 2103, 2104, 2218, 2221,
    2235, 2239, 2240, 2241, 2242, 2243, 2246, 2247, 2249, 2250, 2251, 2296, 2299, 2301,
    2303, 2304, 2305, 2306, 2307, 3121, 2965, 2966, 2967, 2968, 3122, 3123, 2991, 2993,
    3000, 3001, 3002, 3003, 3004, 3016, 3017, 3018, 3019, 3020, 3021, 3022, 3023, 3024,
    3025, 3026, 3027, 3028, 3029, 3030, 3058, 3068, 3072, 3074, 3075, 3077, 3124, 3125,
    3080, 3081, 3088, 3089, 3090, 3091, 3108, 3110, 3113, 3114, 3115, 3116, 3117, 3118,
    3119, 3120,
];

const GENERATED_BATCH10_CODES: &[u32] = &[
    3294, 3126, 3127, 3128, 3129, 3130, 3131, 3132, 3133, 3134, 3135, 3136, 3137, 3138,
    3295, 3140, 3152, 3300, 3328, 3200, 3204, 3205, 3206, 3207, 3208, 3209, 3210, 3211,
    3212, 3213, 3214, 3215, 3216, 3217, 3218, 3219, 3220, 3221, 3222, 3223, 3224, 3225,
    3226, 3227, 3228, 3229, 3230, 3231, 3232, 3233, 3234, 3235, 3236, 3237, 3238, 3239,
    3240, 3241, 3242, 3243, 3244, 3245, 3246, 3247, 3248, 3249, 3250, 3251, 3252, 3253,
    3254, 3255, 3256, 3257, 3258, 3259, 3260, 3261, 3262, 3263, 3264, 3265, 3266, 3267,
    3268, 3269, 3270, 3271, 3272, 3273, 3274, 3337, 3350, 3351, 3352, 3355, 3358, 3360,
    3361, 3362,
];

const GENERATED_BATCH11_CODES: &[u32] = &[
    3363, 3364, 3365, 3377, 3378, 3379, 3380, 3381, 3382, 3383, 3384, 3385, 3386, 3387,
    3388, 3394, 3404, 3407, 3877, 3417, 3418, 3419, 3420, 3421, 3422, 3423, 3424, 3425,
    3426, 3427, 3428, 3429, 3430, 3431, 3432, 3433, 3434, 3435, 3436, 3437, 3438, 3441,
    3442, 3443, 3444, 3445, 3446, 3448, 3451, 3452, 3453, 3455, 3456, 3457, 3458, 3459,
    3460, 3463, 3464, 3553, 3554, 3555, 3556, 3557, 3558, 3559, 3560, 3561, 3562, 3563,
    3564, 3565, 3566, 3567, 3568, 3569, 3570, 3753, 3754, 3755, 3756, 3757, 3758, 3759,
    3760, 3764, 3770, 3794, 3795, 3796, 3827, 3828, 3844, 3851, 3852, 3854, 3873, 3874,
    3875, 3876,
];

const GENERATED_BATCH12_CODES: &[u32] = &[
    3878, 3879, 3880, 3881, 3882, 3883, 3884, 3885, 3893, 3912, 3942, 3943, 3944, 3945,
    3946, 3947, 3948, 3949, 3950, 4093, 4094, 4095, 4096, 4217, 4390, 4391, 4392, 4393,
    4394, 4395, 4396, 4397, 4398, 4399, 4400, 4401, 4402, 4403, 4404, 4405, 4406, 4407,
    4408, 4409, 4410, 4411, 4412, 4413, 4414, 4418, 4419, 4420, 4421, 4422, 4423, 4424,
    4425, 4426, 4427, 4428, 4429, 4430, 4431, 4432, 4433, 4437, 4438, 4439, 4455, 4456,
    4457, 4826, 4839, 5018, 5256, 5257, 5048, 5167, 5168, 5169, 5170, 5171, 5172, 5173,
    5174, 5175, 5176, 5177, 5178, 5180, 5183, 5184, 5185, 5188, 5221, 5234, 5235, 5253,
    5254, 5255,
];

const GENERATED_BATCH13_CODES: &[u32] = &[
    5258, 5259, 5266, 5329, 5330, 5331, 5343, 5344, 5345, 5346, 5347, 5348, 5349, 5367,
    5456, 5457, 5459, 5461, 5462, 5472, 5479, 5480, 5481, 5482, 5518, 5519, 5530, 5550,
    5551, 5552, 5559, 5588, 5589, 5623, 5624, 5625, 5632, 5633, 5634, 5635, 5636, 5637,
    5638, 5639, 5641, 5643, 5646, 5654, 5655, 5825, 5880, 5887, 5921, 5922, 5923, 5924,
    5925, 5926, 5927, 5928, 5929, 5930, 5931, 5932, 5933, 5934, 5935, 5936, 5937, 5938,
    5939, 5940, 6050, 6051, 6052, 6053, 6054, 6055, 6056, 6057, 6058, 6059, 6060, 6061,
    6062, 6063, 6064, 6065, 6066, 6067, 6068, 6069, 6070, 6071, 6072, 6073, 6074, 6075,
    6076, 6077,
];

const GENERATED_BATCH14_CODES: &[u32] = &[
    6078, 6079, 6080, 6081, 6082, 6083, 6084, 6085, 6086, 6087, 6088, 6089, 6090, 6091,
    6092, 6093, 6094, 6095, 6096, 6097, 6098, 6099, 6100, 6101, 6102, 6103, 6104, 6105,
    6106, 6107, 6108, 6109, 6110, 6111, 6112, 6113, 6114, 6115, 6116, 6117, 6118, 6119,
    6120, 6121, 6122, 6123, 6124, 6125, 6128, 6129, 6201, 6202, 6204, 6637, 6646, 6720,
    6721, 6722, 6723, 6785, 6787, 6789, 6791, 6792, 6793, 6794, 6795, 6796, 6797, 6798,
    6799, 6801, 6803, 6804, 6805, 6806, 6807, 6813, 6815, 6817, 6819, 6821, 6823, 6825,
    6827, 6307, 6316, 6351, 6352, 6353, 6354, 6362, 6372, 6391, 6628, 6629, 6630, 6631,
    6632, 6633,
];

const GENERATED_BATCH15_CODES: &[u32] = &[
    6829, 6831, 6833, 6835, 6837, 6839, 6845, 6847, 6849, 6851, 6852, 6853, 6854, 6855,
    6857, 6859, 6861, 6863, 6867, 6879, 6880, 6884, 6885, 6886, 6887, 6922, 6923, 6924,
    6925, 6962, 6966, 6984, 6991, 7119, 7120, 7121, 7122, 7123, 7124, 7125, 7126, 7127,
    7128, 7132, 7142, 7308, 7310, 7312, 7314, 7316, 7318, 7320, 7322, 7324, 7326, 7328,
    7330, 7332, 7334, 7336, 7338, 7340, 7342, 7344, 7346, 7348, 7350, 7352, 7354, 7356,
    7357, 7358, 7359, 7360, 7361, 7362, 7363, 7364, 7365, 7366, 7367, 7368, 7369, 7370,
    7528, 7529, 7530, 7531, 7532, 7533, 7534, 7535, 7536, 7537, 7538, 7539, 7540, 7541,
    7542, 7543,
];

const GENERATED_BATCH16_CODES: &[u32] = &[
    7544, 7545, 7546, 7547, 7548, 7549, 7550, 7551, 7552, 7553, 7554, 7555, 7556, 7557,
    7558, 7559, 7560, 7561, 7562, 7563, 7564, 7565, 7566, 7567, 7568, 7569, 7570, 7571,
    7572, 7573, 7574, 7575, 7576, 7577, 7578, 7579, 7580, 7581, 7582, 7583, 7584, 7585,
    7586, 7587, 7588, 7589, 7590, 7591, 7592, 7593, 7594, 7595, 7596, 7597, 7598, 7599,
    7600, 7601, 7602, 7603, 7604, 7605, 7606, 7607, 7608, 7609, 7610, 7611, 7612, 7613,
    7614, 7615, 7616, 7617, 7618, 7619, 7620, 7621, 7622, 7623, 7624, 7625, 7626, 7627,
    7628, 7629, 7630, 7631, 7632, 7633, 7634, 7635, 7636, 7637, 7638, 7639, 7640, 7641,
    7642, 7643,
];

const GENERATED_BATCH17_CODES: &[u32] = &[
    7644, 7645, 7692, 7693, 7694, 7695, 7696, 7755, 7756, 7757, 7758, 7759, 7760, 7761,
    7762, 7763, 7764, 7765, 7766, 7767, 7768, 7769, 7770, 7771, 7772, 7773, 7774, 7775,
    7776, 7777, 7778, 7779, 7780, 7781, 7782, 7783, 7784, 7785, 7786, 7787, 7794, 7795,
    7801, 7825, 7826, 7827, 7828, 7829, 7830, 7831, 7877, 7882, 7887, 7899, 8013, 8014,
    8015, 8016, 8017, 8018, 8019, 8020, 8021, 8022, 8023, 8024, 8025, 8026, 8027, 8028,
    8029, 8030, 8031, 8032, 8044, 8045, 8065, 8066, 8067, 8068, 8090, 8091, 8092, 8093,
    8095, 8096, 8097, 8098, 8099, 8100, 8101, 8102, 8103, 8104, 8105, 8106, 8107, 8108,
    8109, 8110,
];

const GENERATED_BATCH18_CODES: &[u32] = &[
    8111, 8112, 8113, 8114, 8115, 8116, 8117, 8118, 8119, 8120, 8121, 8122, 8123, 8124,
    8125, 8126, 8127, 8128, 8129, 8130, 8131, 8132, 8133, 8134, 8135, 8136, 8137, 8138,
    8139, 8140, 8141, 8142, 8143, 8144, 8145, 8146, 8147, 8148, 8149, 8150, 8151, 8152,
    8153, 8154, 8155, 8156, 8157, 8158, 8159, 8160, 8161, 8162, 8163, 8164, 8165, 8166,
    8167, 8168, 8169, 8170, 8171, 8172, 8173, 8177, 8179, 8180, 8181, 8182, 8184, 8185,
    8187, 8189, 8191, 8193, 8196, 8197, 8198, 8200, 8201, 8202, 8203, 8204, 8205, 8206,
    8207, 8208, 8209, 8210, 8212, 8213, 8214, 8216, 8218, 8220, 8222, 8224, 8225, 8226,
    8311, 8312,
];

const GENERATED_BATCH19_CODES: &[u32] = &[
    8313, 8314, 8315, 8316, 8317, 8318, 8319, 8320, 8321, 8322, 8323, 8324, 8325, 8326,
    8327, 8328, 8329, 8330, 8331, 8332, 8333, 8334, 8335, 8336, 8337, 8338, 8339, 8340,
    8341, 8342, 8343, 8344, 8345, 8346, 8347, 8348, 8352, 8353, 8379, 8380, 8381, 8382,
    8383, 8384, 8385, 8387, 8391, 8433, 8518, 8519, 8520, 8521, 8522, 8523, 8524, 8525,
    8526, 8527, 8528, 8529, 8531, 8533, 8534, 8535, 8536, 8538, 8539, 8540, 8677, 8678,
    8679, 8686, 8858, 8859, 8908, 9039, 9040, 9141, 9249, 9250, 9252, 9254, 9271, 9272,
    9273, 9284, 9285, 9300, 9311, 9367, 9373, 9377, 9387, 9456, 9498, 9741, 9748, 9749,
    9761, 9766,
];

const GENERATED_BATCH20_CODES: &[u32] = &[
    9821, 9822, 9823, 9824, 9825, 9826, 9827, 9828, 9829, 9830, 9831, 9832, 9833, 9834,
    9835, 9836, 9837, 9838, 9839, 9840, 9841, 9842, 9843, 9844, 9845, 9846, 9847, 9848,
    9849, 9850, 9851, 9852, 9853, 9854, 9855, 9856, 9857, 9858, 9859, 9860, 9861, 9862,
    9863, 9864, 9865, 9869, 9874, 9875, 9880, 9943, 9945, 9947, 9967, 9972, 9977, 10160,
    10183, 10188, 10194, 10199, 10207, 10212, 10217, 10222, 10227, 10235, 10240, 10250,
    10254, 10270, 10275, 10280, 10448, 10449, 10450, 10451, 10452, 10453, 10454, 10455,
    10456, 10457, 10458, 10459, 10460, 10461, 10462, 10463, 10464, 10465, 10471, 10481,
    10516, 10592, 10594, 10596, 10598, 10601, 10603, 10626,
];

const GENERATED_BATCH21_CODES: &[u32] = &[
    10632, 11114, 11115, 11116, 11117, 11118, 20002, 20047,
    20050, 20249, 20250, 20251, 20252, 20253, 20254, 20255,
    20256, 20257, 20258, 20349, 20350, 20351, 20352, 20353,
    20354, 20355, 20356, 20499, 20790, 20791, 21207, 21208,
    21209, 21210, 21211, 21212, 21213, 21214, 21215, 21216,
    21217, 21218, 21219, 21220, 21221, 21222, 21223, 21224,
    21225, 21226, 21227, 21228, 21229, 21230, 21231, 21232,
    21233, 21234, 21235, 21236, 21237, 21238, 21239, 21240,
    21241, 21242, 21243, 21244, 21245, 21246, 21247, 21248,
    21249, 21250, 21251, 21252, 21253, 21254, 21255, 21256,
    21257, 21258, 21259, 21260, 21261, 21262, 21263, 21264,
    21291, 21292, 21307, 21308, 21309, 21310, 21311, 21312,
    21313, 21314, 21315, 21316,
];

const GENERATED_BATCH22_CODES: &[u32] = &[
    21317, 21318, 21319, 21320, 21321, 21322, 21323, 21324,
    21325, 21326, 21327, 21328, 21329, 21330, 21331, 21332,
    21333, 21334, 21335, 21336, 21337, 21338, 21339, 21340,
    21341, 21342, 21343, 21344, 21345, 21346, 21347, 21348,
    21349, 21350, 21351, 21352, 21353, 21354, 21355, 21356,
    21357, 21358, 21359, 21360, 21361, 21362, 21363, 21364,
    21780, 21782, 21896, 21897, 21898, 21899, 22171, 22172,
    22173, 22174, 22175, 22176, 22177, 22181, 22182, 22183,
    22184, 22185, 22186, 22187, 22191, 22192, 22193, 22194,
    22195, 22196, 22197, 22239, 22240, 22243, 22244, 22245,
    22246, 22247, 22248, 22249, 22250, 22391, 22392,
    22639, 22739, 22780, 22991, 22992, 22993, 22994,
    23301, 23302, 23303, 23304,
];

const GENERATED_BATCH23_CODES: &[u32] = &[
    23305, 23306, 23307, 23308, 23309, 23310, 23311, 23312,
    23313, 23314, 23315, 23316, 23317, 23318, 23319, 23320,
    23321, 23322, 23323, 23324, 23325, 23326, 23327, 23328,
    23329, 23330, 23331, 23332, 23333, 24100, 24200, 24370,
    24371, 24372, 24373, 24374, 24375, 24376, 24377, 24378,
    24379, 24380, 24381, 24382, 24383, 24500, 24891, 24892,
    24893, 25000, 25391, 25392, 25393, 25394, 25395, 26191,
    26192, 26194, 26195, 26391, 26392, 26393, 26729, 26730,
    26732, 26733, 26734, 26735, 26736, 26737, 26738, 26739,
    26740, 26741, 26742, 26743, 26744, 26745, 26746, 26748,
    26749, 26750, 26751, 26752, 26753, 26754, 26755, 26756,
    26757, 26758, 26759, 26760, 26766, 26767, 26768, 26769,
    26770, 26771, 26772, 26773,
];

const GENERATED_BATCH24_CODES: &[u32] = &[
    26774, 26775, 26776, 26777, 26778, 26779, 26780, 26781,
    26782, 26783, 26784, 26785, 26786, 26787, 26791, 26792,
    26793, 26794, 26795, 26796, 26797, 26798, 26799, 26847,
    26848, 26849, 26850, 26851, 26852, 26853, 26854, 26855,
    26856, 26857, 26858, 26859, 26860, 26861, 26862, 26863,
    26864, 26865, 26866, 26867, 26868, 26869, 26870, 27205,
    27206, 27207, 27208, 27209, 27210, 27211, 27212, 27213,
    27214, 27215, 27216, 27217, 27218, 27219, 27220, 27221,
    27222, 27223, 27224, 27225, 27226, 27227, 27228, 27229,
    27230, 27231, 27232, 27291, 27292, 27391, 27392, 27393,
    27394, 27395, 27396, 27397, 27398, 27493, 27500, 27574,
    27701, 27702, 27703, 27704, 27705, 27706, 27707, 28191,
    28192, 28193, 28232, 28348,
];

const GENERATED_BATCH25_CODES: &[u32] = &[
    28357, 28358, 28600, 28991, 29101, 29220, 29221, 29333,
    29702, 29738, 29739, 29849, 29850, 29901, 29902, 30161,
    30162, 30163, 30164, 30165, 30166, 30167, 30168, 30169,
    30170, 30171, 30172, 30173, 30174, 30175, 30176, 30177,
    30178, 30179, 30200, 30339, 30340, 30491, 30492, 30493,
    30494, 30729, 30730, 30731, 30732, 30791, 30792, 31028,
    31121, 31154, 31170, 31171, 31251, 31252, 31253, 31259,
    31281, 31282, 31283, 31284, 31285, 31286, 31288, 31289,
    31290, 31528, 31529, 31600, 31838, 31839, 31901,
    31986, 31987, 31988, 31989, 31990, 31991, 31992, 31993,
    31994, 31995, 31996, 31997, 31998, 31999, 32000, 32001,
    32002, 32003, 32005, 32006, 32007, 32008, 32009, 32010,
    32011, 32012, 32013, 32014,
];

const GENERATED_BATCH26_CODES: &[u32] = &[
    32015, 32016, 32017, 32019, 32020, 32021, 32022,
    32023, 32024, 32025, 32026, 32027, 32028, 32030, 32031,
    32033, 32034, 32035, 32037, 32038, 32039, 32040, 32041,
    32042, 32043, 32044, 32045, 32046, 32047, 32048, 32049,
    32050, 32051, 32052, 32053, 32054, 32055, 32056, 32057,
    32058, 32064, 32065, 32066, 32067, 32081, 32082, 32083,
    32084, 32085, 32086, 32098, 32099, 32100, 32104, 32107,
    32108, 32109, 32110, 32111, 32112, 32113, 32114, 32115,
    32116, 32117, 32118, 32119, 32120, 32121, 32122, 32123,
    32124, 32125, 32126, 32127, 32128, 32129, 32130, 32133,
    32134, 32135, 32136, 32137, 32138, 32139, 32140, 32141,
    32142, 32143, 32144, 32145, 32146, 32147, 32148, 32149,
    32150, 32151, 32152, 32153,
];

const GENERATED_BATCH27_CODES: &[u32] = &[
    32154, 32155, 32156, 32157, 32158, 32159, 32161, 32164,
    32165, 32166, 32167, 32181, 32182, 32183, 32184, 32185,
    32186, 32187, 32188, 32189, 32190, 32191, 32192, 32193,
    32194, 32195, 32196, 32197, 32198, 32199, 32664, 32665,
    32666, 32667, 32766,
];

const GENERATED_BATCH28_CODES: &[u32] = &[
    3078, 3079, 3275, 3276, 3277, 3278, 3279, 3280,
    3281, 3282, 3283, 3284, 3285, 3286, 3287, 3288,
    3289, 3290, 3291, 3292, 3293, 3411, 3412, 5041,
    5042, 6244, 6245, 6246, 6247, 6248, 6249, 6250,
    6251, 6252, 6253, 6254, 6255, 6256, 6257, 6258,
    6259, 6260, 6261, 6262, 6263, 6264, 6265, 6266,
    6267, 6268, 6269, 6270, 6271, 6272, 6273, 6274,
    6275, 6808, 6809, 6810, 6811, 6840, 6841, 6842,
    6843, 9354, 26731,
];

const GENERATED_BATCH29_CODES: &[u32] = &[
    3167, 3168, 3375, 3376, 5247, 29871, 29872, 29873,
    29874,
];

const GENERATED_BATCH30_CODES: &[u32] = &[
    2985, 2986,
];

const GENERATED_BATCH31_CODES: &[u32] = &[
    2963, 5017, 8441, 29371, 29373, 29375, 29377, 29379,
    29381, 29383, 29385, 29701,
];

const GENERATED_BATCH32_CODES: &[u32] = &[
    22300, 22700, 31300, 32600, 32700,
];

const GENERATED_BATCH33_CODES: &[u32] = &[
    27200,
];

const GENERATED_BATCH34_CODES: &[u32] = &[
    9895,
];

/// EPSG code resolution behavior for [`from_epsg_with_policy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpsgResolutionPolicy {
    /// Require the requested code to exist in the built-in registry.
    Strict,
    /// If the requested code is unsupported, retry with this fallback EPSG code.
    FallbackToEpsg(u32),
    /// If the requested code is unsupported, retry with EPSG:4326.
    FallbackToWgs84,
    /// If the requested code is unsupported, retry with EPSG:3857.
    FallbackToWebMercator,
}

/// EPSG resolution details returned by [`resolve_epsg_with_policy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpsgResolution {
    /// EPSG code requested by the caller.
    pub requested_code: u32,
    /// EPSG code that was ultimately resolved.
    pub resolved_code: u32,
    /// Whether built-in alias catalog mapping was applied.
    pub used_alias_catalog: bool,
    /// Whether fallback resolution was applied.
    pub used_fallback: bool,
}

/// Explicit catalog entry mapping a legacy/vendor code to a supported EPSG code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpsgAliasEntry {
    /// Legacy/vendor source code.
    pub source_code: u32,
    /// Target supported EPSG code.
    pub target_epsg: u32,
    /// Human-readable note describing the mapping intent.
    pub note: &'static str,
}

const ALIAS_CATALOG: &[EpsgAliasEntry] = &[
    EpsgAliasEntry {
        source_code: 900913,
        target_epsg: 3857,
        note: "legacy Google Web Mercator alias",
    },
    EpsgAliasEntry {
        source_code: 3785,
        target_epsg: 3857,
        note: "deprecated Pseudo-Mercator code",
    },
    EpsgAliasEntry {
        source_code: 102100,
        target_epsg: 3857,
        note: "ESRI Web Mercator alias",
    },
    EpsgAliasEntry {
        source_code: 102113,
        target_epsg: 3857,
        note: "legacy ESRI Web Mercator alias",
    },
];

fn runtime_alias_registry() -> &'static RwLock<HashMap<u32, u32>> {
    static REGISTRY: OnceLock<RwLock<HashMap<u32, u32>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Built-in explicit alias catalog for legacy/vendor CRS codes.
pub fn epsg_alias_catalog() -> &'static [EpsgAliasEntry] {
    ALIAS_CATALOG
}

/// Register a runtime EPSG alias mapping.
///
/// This is intended for organization-specific or legacy code mappings.
/// The target EPSG must be supported by the built-in registry.
pub fn register_epsg_alias(source_code: u32, target_epsg: u32) -> Result<()> {
    if source_code == target_epsg {
        return Err(ProjectionError::invalid_param(
            "source_code/target_epsg",
            "source_code and target_epsg must differ",
        ));
    }

    if build_crs(target_epsg).is_err() {
        return Err(ProjectionError::UnsupportedProjection(format!(
            "cannot register alias to unsupported target EPSG:{target_epsg}"
        )));
    }

    let mut guard = runtime_alias_registry()
        .write()
        .map_err(|_| ProjectionError::DatumError("runtime alias registry lock poisoned".to_string()))?;
    guard.insert(source_code, target_epsg);
    Ok(())
}

/// Unregister a runtime EPSG alias mapping.
///
/// Returns the removed target EPSG when a mapping existed.
pub fn unregister_epsg_alias(source_code: u32) -> Option<u32> {
    let mut guard = runtime_alias_registry().write().ok()?;
    guard.remove(&source_code)
}

/// Remove all runtime EPSG alias mappings.
pub fn clear_runtime_epsg_aliases() {
    if let Ok(mut guard) = runtime_alias_registry().write() {
        guard.clear();
    }
}

/// Return all runtime EPSG alias mappings as `(source_code, target_epsg)` pairs.
pub fn runtime_epsg_aliases() -> Vec<(u32, u32)> {
    let mut out = if let Ok(guard) = runtime_alias_registry().read() {
        guard.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    out.sort_unstable_by_key(|(k, _)| *k);
    out
}

/// Metadata about a registered EPSG entry.
#[derive(Debug, Clone)]
pub struct EpsgInfo {
    /// The EPSG numeric code.
    pub code: u32,
    /// Official CRS name.
    pub name: &'static str,
    /// Brief description of the area of use.
    pub area_of_use: &'static str,
    /// Unit of measure for projected axes ("metre", "degree").
    pub unit: &'static str,
}

/// Build a [`Crs`] from an EPSG code.
///
/// Returns `Err(ProjectionError::UnsupportedProjection)` if the code is not
/// in the built-in registry.
///
/// # Example
///
/// ```rust
/// use wbprojection::epsg::from_epsg;
///
/// // UTM Zone 32N
/// let crs = from_epsg(32632).unwrap();
/// let (easting, northing) = crs.forward(9.0, 48.0).unwrap();
///
/// // Web Mercator
/// let web = from_epsg(3857).unwrap();
///
/// // British National Grid
/// let bng = from_epsg(27700).unwrap();
/// ```
pub fn from_epsg(code: u32) -> Result<Crs> {
    build_crs(code)
}

/// Extract an EPSG code from a WKT string or SRS-style CRS reference.
///
/// This is a lightweight importer that recognizes embedded EPSG authority or
/// identifier markers such as `AUTHORITY["EPSG",4326]`, `ID["EPSG",4326]`,
/// `EPSG:4326`, URN forms, and common HTTP CRS references. It does not parse
/// arbitrary WKT projection parameters into a new CRS definition.
pub fn epsg_from_wkt(wkt: &str) -> Option<u32> {
    extract_epsg_after_marker(wkt, "AUTHORITY[\"EPSG\",")
        .or_else(|| extract_epsg_after_marker(wkt, "ID[\"EPSG\","))
        .or_else(|| {
            let trimmed = wkt.trim();
            if trimmed.contains('[') || trimmed.contains(']') || trimmed.contains('(') {
                None
            } else {
                epsg_from_srs_reference(trimmed)
            }
        })
}

#[derive(Debug, Clone)]
struct CrsCandidate {
    code: u32,
    kind_label: String,
    utm_zone: Option<u8>,
    utm_south: Option<bool>,
    datum_norm: String,
    ellipsoid: Ellipsoid,
    lon0: f64,
    lat0: f64,
    false_easting: f64,
    false_northing: f64,
    scale: f64,
    name_norm: String,
}

static CRS_CANDIDATES: OnceLock<Vec<CrsCandidate>> = OnceLock::new();
const IDENTIFY_MIN_SCORE: f64 = 70.0;
const IDENTIFY_TIE_EPSILON: f64 = 1.0e-9;

/// Matching policy for adaptive WKT/CRS to EPSG identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpsgIdentifyPolicy {
    /// Return the top candidate when it passes confidence threshold.
    Lenient,
    /// Reject ambiguous top matches even if confidence threshold is met.
    Strict,
}

/// Score contribution details for a candidate EPSG match.
#[derive(Debug, Clone)]
pub struct EpsgIdentifyCandidate {
    /// Candidate EPSG code.
    pub code: u32,
    /// Weighted total score in [0, 100].
    pub total_score: f64,
    /// Score contribution from projection-kind match.
    pub kind_score: f64,
    /// Score contribution from datum-name match.
    pub datum_score: f64,
    /// Score contribution from ellipsoid match.
    pub ellipsoid_score: f64,
    /// Score contribution from UTM zone/hemisphere match.
    pub zone_score: f64,
    /// Score contribution from numeric projection-parameter proximity.
    pub parameter_score: f64,
    /// Score contribution from CRS-name similarity.
    pub name_score: f64,
}

/// Rich report for adaptive WKT/CRS to EPSG identification.
#[derive(Debug, Clone)]
pub struct EpsgIdentifyReport {
    /// Resolved EPSG code when identification succeeds.
    pub resolved_code: Option<u32>,
    /// Whether the winning candidate passed confidence threshold.
    pub passed_threshold: bool,
    /// Whether multiple top candidates are tied (strict mode rejects).
    pub ambiguous: bool,
    /// Whether an embedded/explicit EPSG value was used directly.
    pub used_embedded_epsg: bool,
    /// Top scored candidates in descending order.
    pub top_candidates: Vec<EpsgIdentifyCandidate>,
}

fn crs_candidates() -> &'static [CrsCandidate] {
    CRS_CANDIDATES.get_or_init(|| {
        let mut out = Vec::<CrsCandidate>::new();
        for code in known_epsg_codes() {
            if generated_epsg_wkt(code).is_some() {
                continue;
            }
            let Ok(crs) = build_crs(code) else {
                continue;
            };
            let p = crs.projection.params();
            let (zone, south) = match p.kind {
                ProjectionKind::Utm { zone, south } => (Some(zone), Some(south)),
                _ => (None, None),
            };

            out.push(CrsCandidate {
                code,
                kind_label: projection_kind_label(&p.kind),
                utm_zone: zone,
                utm_south: south,
                datum_norm: normalize_datum_name(crs.datum.name),
                ellipsoid: crs.datum.ellipsoid,
                lon0: p.lon0,
                lat0: p.lat0,
                false_easting: p.false_easting,
                false_northing: p.false_northing,
                scale: p.scale,
                name_norm: normalize_name(strip_epsg_suffix(&crs.name)),
            });
        }
        out
    })
}

/// Identify the best supported EPSG code for a WKT CRS definition.
///
/// This function is designed to adapt as support expands:
/// it compares a parsed CRS against fingerprints built from
/// [`known_epsg_codes()`] rather than hardcoded one-off mappings.
pub fn identify_epsg_from_wkt(wkt: &str) -> Option<u32> {
    identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient)
}

/// Identify the best supported EPSG code for a WKT CRS definition using
/// explicit match policy.
pub fn identify_epsg_from_wkt_with_policy(wkt: &str, policy: EpsgIdentifyPolicy) -> Option<u32> {
    identify_epsg_from_wkt_report(wkt, policy).and_then(|r| r.resolved_code)
}

/// Return a scored identification report for a WKT CRS definition.
pub fn identify_epsg_from_wkt_report(wkt: &str, policy: EpsgIdentifyPolicy) -> Option<EpsgIdentifyReport> {
    if let Some(code) = epsg_from_wkt(wkt) {
        return Some(EpsgIdentifyReport {
            resolved_code: Some(code),
            passed_threshold: true,
            ambiguous: false,
            used_embedded_epsg: true,
            top_candidates: vec![EpsgIdentifyCandidate {
                code,
                total_score: 999.0,
                kind_score: 0.0,
                datum_score: 0.0,
                ellipsoid_score: 0.0,
                zone_score: 0.0,
                parameter_score: 0.0,
                name_score: 0.0,
            }],
        });
    }

    let norm_wkt = normalize_name(wkt);
    let datum_hint = datum_hint_from_wkt_text(&norm_wkt);
    let ellipsoid_hint = ellipsoid_hint_from_wkt_text(&norm_wkt);
    let name_hint = crs_name_hint_from_wkt_text(wkt).map(|s| normalize_name(&s));

    let parsed = crate::wkt::parse_crs_from_wkt(wkt).ok()?;
    Some(identify_epsg_from_crs_report_internal(
        &parsed,
        policy,
        datum_hint,
        ellipsoid_hint,
        name_hint,
    ))
}

/// Identify the best supported EPSG code for an already-parsed CRS.
pub fn identify_epsg_from_crs(crs: &Crs) -> Option<u32> {
    identify_epsg_from_crs_with_policy(crs, EpsgIdentifyPolicy::Lenient)
}

/// Identify the best supported EPSG code for an already-parsed CRS using
/// explicit match policy.
pub fn identify_epsg_from_crs_with_policy(crs: &Crs, policy: EpsgIdentifyPolicy) -> Option<u32> {
    identify_epsg_from_crs_report(crs, policy).resolved_code
}

/// Return a scored identification report for an already-parsed CRS.
pub fn identify_epsg_from_crs_report(crs: &Crs, policy: EpsgIdentifyPolicy) -> EpsgIdentifyReport {
    identify_epsg_from_crs_report_internal(crs, policy, None, None, None)
}

fn identify_epsg_from_crs_report_internal(
    crs: &Crs,
    policy: EpsgIdentifyPolicy,
    datum_hint: Option<String>,
    ellipsoid_hint: Option<String>,
    name_hint: Option<String>,
) -> EpsgIdentifyReport {
    let p = crs.projection.params();
    let src_kind = projection_kind_label(&p.kind);
    let mut src_datum = normalize_datum_name(crs.datum.name);
    if let Some(h) = datum_hint {
        if src_datum.is_empty() || !src_datum.contains(&h) {
            src_datum = h;
        }
    }
    let src_name = normalize_name(strip_epsg_suffix(&crs.name));
    let src_name_for_match = if src_name.is_empty() {
        name_hint.as_deref().unwrap_or("")
    } else {
        &src_name
    };
    let src_name_hint = name_hint.as_deref();
    let (src_zone, src_south) = match p.kind {
        ProjectionKind::Utm { zone, south } => (Some(zone), Some(south)),
        _ => (None, None),
    };
    let (name_zone_hint, name_south_hint) = extract_utm_zone_hint(&src_name).unwrap_or((0, false));
    let has_name_zone_hint = extract_utm_zone_hint(&src_name).is_some();

    // Phase 1: strict structural match for high-confidence identification.
    let mut structural = Vec::<&CrsCandidate>::new();
    for c in crs_candidates() {
        if c.kind_label != src_kind {
            continue;
        }
        if c.datum_norm != src_datum {
            continue;
        }
        if c.ellipsoid != crs.datum.ellipsoid {
            continue;
        }
        if src_zone.is_some() {
            if c.utm_zone != src_zone || c.utm_south != src_south {
                continue;
            }
        }
        if params_close(c, p, 1.0e-8, 1.0e-4, 1.0e-10) {
            structural.push(c);
        }
    }

    if !structural.is_empty() {
        let mut structural_scored = structural
            .into_iter()
            .map(|c| {
                let mut name_score = if !src_name_for_match.is_empty()
                    && (c.name_norm.contains(src_name_for_match) || src_name_for_match.contains(&c.name_norm))
                {
                    10.0
                } else {
                    0.0
                };
                if src_zone.is_none()
                    && datum_family(&src_datum) == Some("wgs84")
                    && c.code == 4326
                {
                    name_score += 20.0;
                }
                EpsgIdentifyCandidate {
                    code: c.code,
                    total_score: 200.0 + name_score,
                    kind_score: 40.0,
                    datum_score: 20.0,
                    ellipsoid_score: 10.0,
                    zone_score: if src_zone.is_some() { 30.0 } else { 0.0 },
                    parameter_score: 100.0,
                    name_score,
                }
            })
            .collect::<Vec<_>>();

        structural_scored.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.code.cmp(&a.code))
        });

        return EpsgIdentifyReport {
            resolved_code: structural_scored.first().map(|c| c.code),
            passed_threshold: true,
            ambiguous: false,
            used_embedded_epsg: false,
            top_candidates: structural_scored.into_iter().take(5).collect(),
        };
    }

    // Phase 2+3: score all candidates and rank, with deterministic tie-break.
    let mut scored = Vec::<EpsgIdentifyCandidate>::new();
    for c in crs_candidates() {
        let candidate = score_candidate(
            c,
            p,
            &src_kind,
            &src_datum,
            src_name_for_match,
            src_zone,
            src_south,
            if has_name_zone_hint { Some(name_zone_hint) } else { None },
            if has_name_zone_hint { Some(name_south_hint) } else { None },
            crs.datum.ellipsoid.clone(),
            ellipsoid_hint.as_deref(),
            src_name_hint,
        );
        scored.push(candidate);
    }

    scored.sort_by(|a, b| {
        b.total_score
            .partial_cmp(&a.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.code.cmp(&a.code))
    });

    let best = scored.first();
    let best_score = best.map(|c| c.total_score).unwrap_or(f64::NEG_INFINITY);
    let passed_threshold = best_score >= IDENTIFY_MIN_SCORE;
    let ambiguous = if scored.len() >= 2 {
        let second = &scored[1];
        passed_threshold && (best_score - second.total_score).abs() <= IDENTIFY_TIE_EPSILON
    } else {
        false
    };

    let resolved_code = match policy {
        EpsgIdentifyPolicy::Lenient => {
            if passed_threshold {
                best.map(|c| c.code)
            } else {
                None
            }
        }
        EpsgIdentifyPolicy::Strict => {
            if passed_threshold && !ambiguous {
                best.map(|c| c.code)
            } else {
                None
            }
        }
    };

    EpsgIdentifyReport {
        resolved_code,
        passed_threshold,
        ambiguous,
        used_embedded_epsg: false,
        top_candidates: scored.into_iter().take(5).collect(),
    }
}

fn projection_kind_label(kind: &ProjectionKind) -> String {
    // Debug output starts with stable enum variant label; this avoids a large
    // exhaustive matcher and automatically includes newly-added variants.
    let debug = format!("{kind:?}");
    debug.split_whitespace().next().unwrap_or("Unknown").to_string()
}

fn strip_epsg_suffix(name: &str) -> &str {
    if let Some(idx) = name.rfind(" (EPSG:") {
        &name[..idx]
    } else {
        name
    }
}

fn normalize_name(name: &str) -> String {
    name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn normalize_datum_name(name: &str) -> String {
    let mut n = normalize_name(name);

    if let Some(rest) = n.strip_prefix('d') {
        if !rest.is_empty() {
            n = rest.to_string();
        }
    }

    // Fold common long-form WKT datum names to common short forms.
    n = n.replace("northamerican1983", "nad83");
    n = n.replace("northamerican1927", "nad27");
    n = n.replace("worldgeodeticsystem1984", "wgs84");
    n = n.replace("worldgeodeticsystem1972", "wgs72");
    n = n.replace("wgs1984", "wgs84");
    n = n.replace("wgs1972", "wgs72");
    n = n.replace("gda1994", "gda94");
    n = n.replace("gda2020", "gda2020");
    n = n.replace("nzgd2000", "nzgd2000");
    n = n.replace("sirgas2000", "sirgas2000");

    n
}

fn datum_family(name: &str) -> Option<&'static str> {
    if name.contains("nad83csrs") {
        Some("nad83csrs")
    } else if name.contains("nad83") {
        Some("nad83")
    } else if name.contains("nad27") {
        Some("nad27")
    } else if name.contains("wgs84") {
        Some("wgs84")
    } else if name.contains("wgs72") {
        Some("wgs72")
    } else if name.contains("etrs89") {
        Some("etrs89")
    } else if name.contains("gda2020") {
        Some("gda2020")
    } else if name.contains("gda94") {
        Some("gda94")
    } else if name.contains("nzgd2000") {
        Some("nzgd2000")
    } else if name.contains("sirgas2000") {
        Some("sirgas2000")
    } else {
        None
    }
}

fn datum_hint_from_wkt_text(norm_wkt: &str) -> Option<String> {
    if norm_wkt.contains("nad83csrs") || norm_wkt.contains("northamerican1983csrs") {
        Some("nad83csrs".to_string())
    } else if norm_wkt.contains("nad83") || norm_wkt.contains("northamerican1983") {
        Some("nad83".to_string())
    } else if norm_wkt.contains("nad27") || norm_wkt.contains("northamerican1927") {
        Some("nad27".to_string())
    } else if norm_wkt.contains("wgs84") || norm_wkt.contains("wgs1984") {
        Some("wgs84".to_string())
    } else if norm_wkt.contains("wgs72") || norm_wkt.contains("wgs1972") {
        Some("wgs72".to_string())
    } else if norm_wkt.contains("etrs89") {
        Some("etrs89".to_string())
    } else if norm_wkt.contains("gda2020") {
        Some("gda2020".to_string())
    } else if norm_wkt.contains("gda1994") || norm_wkt.contains("gda94") {
        Some("gda94".to_string())
    } else if norm_wkt.contains("nzgd2000") || norm_wkt.contains("nzgd2000") {
        Some("nzgd2000".to_string())
    } else if norm_wkt.contains("sirgas2000") {
        Some("sirgas2000".to_string())
    } else {
        None
    }
}

fn ellipsoid_family_from_norm_name(norm_name: &str) -> Option<&'static str> {
    if norm_name.contains("grs80") {
        Some("grs80")
    } else if norm_name.contains("wgs84") || norm_name.contains("wgs1984") {
        Some("wgs84")
    } else if norm_name.contains("wgs72") || norm_name.contains("wgs1972") {
        Some("wgs72")
    } else if norm_name.contains("clarke1866") {
        Some("clarke1866")
    } else {
        None
    }
}

fn ellipsoid_hint_from_wkt_text(norm_wkt: &str) -> Option<String> {
    if norm_wkt.contains("grs1980") {
        Some("grs80".to_string())
    } else if norm_wkt.contains("wgs1984") || norm_wkt.contains("wgs84") {
        Some("wgs84".to_string())
    } else if norm_wkt.contains("wgs1972") || norm_wkt.contains("wgs72") {
        Some("wgs72".to_string())
    } else if norm_wkt.contains("clarke1866") {
        Some("clarke1866".to_string())
    } else {
        None
    }
}

fn crs_name_hint_from_wkt_text(wkt: &str) -> Option<String> {
    let start = wkt.find('"')? + 1;
    let rest = &wkt[start..];
    let end_rel = rest.find('"')?;
    let name = rest[..end_rel].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_utm_zone_hint(norm_name: &str) -> Option<(u8, bool)> {
    let zone_pos = norm_name.find("zone")?;
    let rest = &norm_name[zone_pos + 4..];
    let digit_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_len == 0 {
        return None;
    }
    let zone = rest.get(..digit_len)?.parse::<u8>().ok()?;
    if !(1..=60).contains(&zone) {
        return None;
    }

    let hemi = rest.get(digit_len..digit_len + 1).unwrap_or("n");
    let south = hemi == "s";
    Some((zone, south))
}

fn projection_kinds_compatible(src_kind: &str, candidate_kind: &str) -> bool {
    (src_kind == "TransverseMercator" && candidate_kind == "Utm")
        || (src_kind == "Utm" && candidate_kind == "TransverseMercator")
}

fn params_close(c: &CrsCandidate, p: &ProjectionParams, angle_tol: f64, meter_tol: f64, scale_tol: f64) -> bool {
    (c.lon0 - p.lon0).abs() <= angle_tol
        && (c.lat0 - p.lat0).abs() <= angle_tol
        && (c.false_easting - p.false_easting).abs() <= meter_tol
        && (c.false_northing - p.false_northing).abs() <= meter_tol
        && (c.scale - p.scale).abs() <= scale_tol
}

fn score_candidate(
    c: &CrsCandidate,
    p: &ProjectionParams,
    src_kind: &str,
    src_datum: &str,
    src_name: &str,
    src_zone: Option<u8>,
    src_south: Option<bool>,
    src_name_zone_hint: Option<u8>,
    src_name_south_hint: Option<bool>,
    src_ellipsoid: Ellipsoid,
    src_ellipsoid_hint: Option<&str>,
    src_name_hint: Option<&str>,
) -> EpsgIdentifyCandidate {
    let mut kind_score = 0.0;
    let mut datum_score = 0.0;
    let mut ellipsoid_score = 0.0;
    let mut zone_score = 0.0;
    let mut parameter_score = 0.0;
    let mut name_score = 0.0;

    if c.kind_label == src_kind {
        kind_score += 40.0;
    } else if projection_kinds_compatible(src_kind, &c.kind_label) {
        kind_score += 30.0;
    }

    if c.datum_norm == src_datum {
        datum_score += 20.0;
    } else if !src_datum.is_empty() {
        match (datum_family(src_datum), datum_family(&c.datum_norm)) {
            (Some(sf), Some(cf)) if sf == cf => datum_score += 12.0,
            (Some("nad83"), Some("nad83csrs")) | (Some("nad83csrs"), Some("nad83")) => datum_score += 8.0,
            (Some(_), Some(_)) => datum_score -= 20.0,
            _ => {}
        }
    }

    if c.ellipsoid == src_ellipsoid {
        ellipsoid_score += 10.0;
    } else if let Some(hint) = src_ellipsoid_hint {
        if let Some(cf) = ellipsoid_family_from_norm_name(&normalize_name(c.ellipsoid.name)) {
            if cf == hint {
                ellipsoid_score += 8.0;
            } else {
                ellipsoid_score -= 6.0;
            }
        }
    }

    if let Some(zone) = src_zone {
        if c.utm_zone == Some(zone) {
            zone_score += 20.0;
        }
        if c.utm_south == src_south {
            zone_score += 10.0;
        }
    }

    if let Some(zone) = src_name_zone_hint {
        if c.utm_zone == Some(zone) {
            zone_score += 20.0;
        }
        if c.utm_south == src_name_south_hint {
            zone_score += 10.0;
        }
    }

    // Param similarity reward with practical tolerances.
    let lon_diff = (c.lon0 - p.lon0).abs();
    let lat_diff = (c.lat0 - p.lat0).abs();
    let fe_diff = (c.false_easting - p.false_easting).abs();
    let fn_diff = (c.false_northing - p.false_northing).abs();
    let k_diff = (c.scale - p.scale).abs();

    if lon_diff <= 1.0e-3 {
        parameter_score += 7.0;
    }
    if lat_diff <= 1.0e-3 {
        parameter_score += 7.0;
    }
    if fe_diff <= 10.0 {
        parameter_score += 5.0;
    }
    if fn_diff <= 10.0 {
        parameter_score += 5.0;
    }
    if k_diff <= 1.0e-6 {
        parameter_score += 6.0;
    }

    // Name-level tie-breaker.
    if !src_name.is_empty() && (c.name_norm.contains(src_name) || src_name.contains(&c.name_norm)) {
        name_score += 8.0;
    }

    if let Some(hint) = src_name_hint {
        if hint.contains("greekgrid") {
            if c.name_norm.contains("greekgrid") {
                name_score += 14.0;
            } else {
                name_score -= 4.0;
            }
        }
        if hint.contains("etrs") {
            if c.name_norm.contains("etrs") {
                name_score += 10.0;
            } else {
                name_score -= 6.0;
            }
        }
        if hint.contains("britishnationalgrid") {
            if c.name_norm.contains("britishnationalgrid") || c.name_norm.contains("osgb") {
                name_score += 10.0;
            }
        }
        if hint.contains("portugaltm06") || hint.contains("tm06") {
            if c.name_norm.contains("tm06") || c.name_norm.contains("portugal") {
                name_score += 10.0;
            }
        }
    }

    if src_name.contains("csrs") {
        let src_has_version = src_name.contains("csrsv");
        let cand_has_version = c.name_norm.contains("csrsv");
        if !src_has_version && cand_has_version {
            name_score -= 6.0;
        }
        if !src_has_version && !cand_has_version && c.name_norm.contains("nad83csrs") {
            name_score += 6.0;
        }
    }

    EpsgIdentifyCandidate {
        code: c.code,
        total_score: kind_score + datum_score + ellipsoid_score + zone_score + parameter_score + name_score,
        kind_score,
        datum_score,
        ellipsoid_score,
        zone_score,
        parameter_score,
        name_score,
    }
}

/// Extract an EPSG code from a CRS reference string.
///
/// Supported inputs include plain numeric codes, `EPSG:xxxx`, OGC URNs, and
/// common `opengis.net` CRS URLs.
pub fn epsg_from_srs_reference(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(code) = trimmed.parse::<u32>() {
        return Some(code);
    }

    let upper = trimmed.to_ascii_uppercase();
    if !upper.contains("EPSG") {
        return None;
    }

    let mut last_digits: Option<u32> = None;
    let mut start: Option<usize> = None;
    for (idx, ch) in upper.char_indices() {
        if ch.is_ascii_digit() {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(sidx) = start.take() {
            if let Ok(code) = upper[sidx..idx].parse::<u32>() {
                last_digits = Some(code);
            }
        }
    }
    if let Some(sidx) = start {
        if let Ok(code) = upper[sidx..].parse::<u32>() {
            last_digits = Some(code);
        }
    }

    last_digits
}

/// Build a [`Crs`] from a WKT string or SRS-style CRS reference that embeds an EPSG code.
///
/// This helper first tries [`epsg_from_wkt`] to discover an EPSG identifier and
/// resolve through the built-in registry. If the WKT has no embedded EPSG code,
/// it falls back to the crate's internal WKT parser for common WKT1/WKT2
/// geographic and projected CRS definitions.
pub fn from_wkt(wkt: &str) -> Result<Crs> {
    if let Some(code) = epsg_from_wkt(wkt) {
        from_epsg(code)
    } else {
        crate::wkt::parse_crs_from_wkt(wkt)
    }
}

/// Build a [`CompoundCrs`] from a WKT compound CRS definition.
///
/// Supports common WKT1 `COMPD_CS[...]` and WKT2 `COMPOUNDCRS[...]` forms
/// with a horizontal and vertical component.
pub fn compound_from_wkt(wkt: &str) -> Result<CompoundCrs> {
    if let Some(code) = epsg_from_wkt(wkt) {
        return CompoundCrs::from_epsg(code);
    }
    crate::wkt::parse_compound_crs_from_wkt(wkt)
}

/// Build a [`Crs`] from an EPSG code using explicit resolution policy.
///
/// This enables opt-in fallback behavior for unsupported codes.
pub fn from_epsg_with_policy(code: u32, policy: EpsgResolutionPolicy) -> Result<Crs> {
    let resolved = resolve_epsg_with_policy(code, policy)?;
    build_crs(resolved.resolved_code)
}

/// Build a [`Crs`] from an EPSG code using explicit alias catalog + policy.
///
/// Resolution order:
/// 1) exact EPSG support,
/// 2) built-in alias catalog,
/// 3) fallback policy.
pub fn from_epsg_with_catalog(code: u32, policy: EpsgResolutionPolicy) -> Result<Crs> {
    let resolved = resolve_epsg_with_catalog(code, policy)?;
    build_crs(resolved.resolved_code)
}

/// Resolve an EPSG code using a policy without constructing a [`Crs`].
pub fn resolve_epsg_with_policy(code: u32, policy: EpsgResolutionPolicy) -> Result<EpsgResolution> {
    if build_crs(code).is_ok() {
        return Ok(EpsgResolution {
            requested_code: code,
            resolved_code: code,
            used_alias_catalog: false,
            used_fallback: false,
        });
    }

    let fallback_code = match policy {
        EpsgResolutionPolicy::Strict => {
            return Err(ProjectionError::UnsupportedProjection(format!(
                "EPSG:{code} is not currently supported"
            )));
        }
        EpsgResolutionPolicy::FallbackToEpsg(c) => c,
        EpsgResolutionPolicy::FallbackToWgs84 => 4326,
        EpsgResolutionPolicy::FallbackToWebMercator => 3857,
    };

    if build_crs(fallback_code).is_ok() {
        Ok(EpsgResolution {
            requested_code: code,
            resolved_code: fallback_code,
            used_alias_catalog: false,
            used_fallback: true,
        })
    } else {
        Err(ProjectionError::UnsupportedProjection(format!(
            "EPSG:{code} is not supported and fallback EPSG:{fallback_code} is also unsupported"
        )))
    }
}

fn extract_epsg_after_marker(wkt: &str, marker: &str) -> Option<u32> {
    let upper = wkt.to_ascii_uppercase();
    let idx = upper.find(marker)?;
    let tail = &upper[idx + marker.len()..];

    let start = tail.find(|c: char| c.is_ascii_digit())?;
    let digits: String = tail[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}

/// Resolve an EPSG code using explicit alias catalog and fallback policy.
pub fn resolve_epsg_with_catalog(code: u32, policy: EpsgResolutionPolicy) -> Result<EpsgResolution> {
    if build_crs(code).is_ok() {
        return Ok(EpsgResolution {
            requested_code: code,
            resolved_code: code,
            used_alias_catalog: false,
            used_fallback: false,
        });
    }

    if let Some(alias_target) = runtime_alias_target_epsg(code).or_else(|| built_in_alias_target_epsg(code)) {
        if build_crs(alias_target).is_ok() {
            return Ok(EpsgResolution {
                requested_code: code,
                resolved_code: alias_target,
                used_alias_catalog: true,
                used_fallback: false,
            });
        }
    }

    let mut base = resolve_epsg_with_policy(code, policy)?;
    base.used_alias_catalog = false;
    Ok(base)
}

fn built_in_alias_target_epsg(code: u32) -> Option<u32> {
    ALIAS_CATALOG
        .iter()
        .find(|e| e.source_code == code)
        .map(|e| e.target_epsg)
}

fn runtime_alias_target_epsg(code: u32) -> Option<u32> {
    let guard = runtime_alias_registry().read().ok()?;
    guard.get(&code).copied()
}

/// Look up metadata for an EPSG code without constructing the full [`Crs`].
///
/// Useful for displaying the name and area of use before projecting.
pub fn epsg_info(code: u32) -> Option<EpsgInfo> {
    get_info(code)
}

/// Return a canonical vertical offset grid name associated with an EPSG vertical CRS code.
///
/// This mapping is used by automatic Vertical<->Vertical transforms when registered
/// vertical offset grids are available in the runtime registry.
pub fn vertical_offset_grid_name(code: u32) -> Option<&'static str> {
    match code {
        3855 => Some("egm2008"),
        5773 => Some("egm96"),
        5701 => Some("osgm15"),
        5702 => Some("vertcon_ngvd29"),
        5703 => Some("geoid18"),
        8228 => Some("geoid18"),
        5714 => Some("msl"),
        5715 => Some("msl"),
        7841 => Some("ausgeoid2020"),
        _ => None,
    }
}

/// List all EPSG codes known to this registry.
pub fn known_epsg_codes() -> Vec<u32> {
    let mut codes: Vec<u32> = NAMED_ENTRIES.iter().map(|e| e.0).collect();
    // UTM ranges are handled dynamically, add those too
    for z in 1u32..=60 {
        codes.push(32200 + z); // WGS72 UTM N
        codes.push(32300 + z); // WGS72 UTM S
        codes.push(32400 + z); // WGS72BE UTM N
        codes.push(32500 + z); // WGS72BE UTM S
        codes.push(32600 + z); // WGS84 UTM N
        codes.push(32700 + z); // WGS84 UTM S
        codes.push(25800 + z); // ETRS89 UTM N
        codes.push(23000 + z); // ED50 UTM N
    }
    // Pulkovo 1942 / 1995 Gauss-Kruger families and neighboring outliers
    for c in 2494u32..=2758 {
        codes.push(c);
    }
    // Pulkovo 1995 / Gauss-Kruger CM family and 6-degree zone families
    for c in 2463u32..=2491 {
        codes.push(c);
    }
    for c in 20004u32..=20032 {
        codes.push(c);
    }
    for c in 28404u32..=28432 {
        codes.push(c);
    }
    // Additional adjusted Pulkovo GK families (active EPSG codes)
    for c in [
        3329u32, 3330, 3331, 3332, 3333, 3334, 3335,
        4417, 4434,
        5631, 5663, 5664, 5665,
        5670, 5671, 5672, 5673, 5674, 5675,
    ] {
        codes.push(c);
    }
    // NAD83(NSRS2007)/state-plane and related families
    for c in 3580u32..=3751 {
        codes.push(c);
    }
    for c in 2334u32..=2390 {
        codes.push(c); // Xian 1980 GK family block
    }
    for z in 1u32..=23 {
        codes.push(26900 + z); // NAD83 UTM N
    }
    // NAD83(2011) UTM N (active EPSG set)
    codes.push(6328); // zone 59N
    codes.push(6329); // zone 60N
    for z in 1u32..=19 {
        codes.push(6329 + z); // 6330..6348 => zones 1N..19N
    }
    for z in 1u32..=22 {
        codes.push(26700 + z); // NAD27 UTM N
    }
    // NAD83(CSRS) UTM (active v1 set)
    for c in [2955u32, 2956, 2957, 2958, 2959, 2960, 2961, 2962, 3154, 3155, 3156, 3157, 3158, 3159, 3160, 3761, 9709, 9713] {
        codes.push(c);
    }
    // NAD83(CSRS) realization families (v2-v8)
    for c in 22207u32..=22222 {
        codes.push(c);
    }
    for c in 22307u32..=22322 {
        codes.push(c);
    }
    for c in 22407u32..=22422 {
        codes.push(c);
    }
    for c in 22607u32..=22622 {
        codes.push(c);
    }
    for c in 22707u32..=22722 {
        codes.push(c);
    }
    for c in 22807u32..=22822 {
        codes.push(c);
    }
    // SIRGAS2000 UTM (active EPSG set)
    for z in 11u32..=22 {
        codes.push(31954 + z); // 31965..31976 (N)
    }
    for z in 17u32..=25 {
        codes.push(31960 + z); // 31977..31985 (S)
    }
    codes.push(6210); // zone 23N
    codes.push(6211); // zone 24N
    codes.push(5396); // zone 26S

    // SAD69 UTM (active EPSG set only)
    codes.push(5463); // zone 17N
    for z in 18u32..=22 {
        codes.push(29150 + z); // 29168..29172 (N)
    }
    for z in 17u32..=25 {
        codes.push(29170 + z); // 29187..29195 (S)
    }

    // PSAD56 UTM
    for z in 17u32..=21 {
        codes.push(24800 + z); // 24817..24821 (N)
    }
    for z in 17u32..=22 {
        codes.push(24860 + z); // 24877..24882 (S)
    }

    // GDA2020 MGA zones 49–56
    for z in 49u32..=56 {
        codes.push(7800 + z); // 7849..7856
    }
    // Legacy workflows parity ranges requested in step 2 then step 1
    for c in 2391u32..=2396 {
        codes.push(c);
    }
    for c in 2400u32..=2442 {
        codes.push(c);
    }
    for c in 2867u32..=2888 {
        codes.push(c);
    }
    for c in 2891u32..=2954 {
        codes.push(c);
    }
    for c in 4120u32..=4147 {
        codes.push(c);
    }
    for c in 4149u32..=4151 {
        codes.push(c);
    }
    for c in 4153u32..=4166 {
        codes.push(c);
    }
    for c in 4168u32..=4176 {
        codes.push(c);
    }
    for c in 4178u32..=4185 {
        codes.push(c);
    }
    codes.extend_from_slice(GENERATED_BATCH1_CODES);
    codes.extend_from_slice(GENERATED_BATCH2_CODES);
    codes.extend_from_slice(GENERATED_BATCH3_CODES);
    codes.extend_from_slice(GENERATED_BATCH4_CODES);
    codes.extend_from_slice(GENERATED_BATCH5_CODES);
    codes.extend_from_slice(GENERATED_BATCH6_CODES);
    codes.extend_from_slice(GENERATED_BATCH7_CODES);
    codes.extend_from_slice(GENERATED_BATCH8_CODES);
    codes.extend_from_slice(GENERATED_BATCH9_CODES);
    codes.extend_from_slice(GENERATED_BATCH10_CODES);
    codes.extend_from_slice(GENERATED_BATCH11_CODES);
    codes.extend_from_slice(GENERATED_BATCH12_CODES);
    codes.extend_from_slice(GENERATED_BATCH13_CODES);
    codes.extend_from_slice(GENERATED_BATCH14_CODES);
    codes.extend_from_slice(GENERATED_BATCH15_CODES);
    codes.extend_from_slice(GENERATED_BATCH16_CODES);
    codes.extend_from_slice(GENERATED_BATCH17_CODES);
    codes.extend_from_slice(GENERATED_BATCH18_CODES);
    codes.extend_from_slice(GENERATED_BATCH19_CODES);
    codes.extend_from_slice(GENERATED_BATCH20_CODES);
    codes.extend_from_slice(GENERATED_BATCH21_CODES);
    codes.extend_from_slice(GENERATED_BATCH22_CODES);
    codes.extend_from_slice(GENERATED_BATCH23_CODES);
    codes.extend_from_slice(GENERATED_BATCH24_CODES);
    codes.extend_from_slice(GENERATED_BATCH25_CODES);
    codes.extend_from_slice(GENERATED_BATCH26_CODES);
    codes.extend_from_slice(GENERATED_BATCH27_CODES);
    codes.extend_from_slice(GENERATED_BATCH28_CODES);
    codes.extend_from_slice(GENERATED_BATCH29_CODES);
    codes.extend_from_slice(GENERATED_BATCH30_CODES);
    codes.extend_from_slice(GENERATED_BATCH31_CODES);
    codes.extend_from_slice(GENERATED_BATCH32_CODES);
    codes.extend_from_slice(GENERATED_BATCH33_CODES);
    codes.extend_from_slice(GENERATED_BATCH34_CODES);
    codes.sort_unstable();
    codes.dedup();
    codes
}

/// Generate an ESRI-formatted WKT representation for an EPSG code.
///
/// Returns `Err(ProjectionError::UnsupportedProjection)` if the code is not
/// in the built-in registry.
pub fn to_esri_wkt(code: u32) -> Result<String> {
    let crs = build_crs(code)?;
    let params = crs.projection.params();

    let datum = &crs.datum;
    let ellipsoid = &datum.ellipsoid;
    let inv_f = if ellipsoid.f.abs() < 1e-15 {
        0.0
    } else {
        1.0 / ellipsoid.f
    };

    let geogcs = format!(
        "GEOGCS[\"GCS_{}\",DATUM[\"{}\",SPHEROID[\"{}\",{:.3},{:.9}]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]",
        datum.name,
        datum.name,
        ellipsoid.name,
        ellipsoid.a,
        inv_f
    );

    let projcs_name = crs.name.replace('"', "'");

    if is_geocentric_epsg(code) {
        return Ok(format!(
            "GEOCCS[\"{}\",DATUM[\"{}\",SPHEROID[\"{}\",{:.3},{:.9}]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Meter\",1.0]]",
            projcs_name,
            datum.name,
            ellipsoid.name,
            ellipsoid.a,
            inv_f
        ));
    }

    if is_vertical_epsg(code) {
        let (axis_name, axis_dir) = vertical_axis_spec(code);
        let (unit_name, unit_scale) = vertical_wkt_unit(code, true);
        return Ok(format!(
            "VERT_CS[\"{}\",VERT_DATUM[\"{}\",2005],UNIT[\"{}\",{}],AXIS[\"{}\",{}]]",
            projcs_name,
            datum.name,
            unit_name,
            unit_scale,
            axis_name,
            axis_dir
        ));
    }

    if is_geographic_epsg(code) {
        // Geographic CRS: geogcs is already a complete GEOGCS[...] string.
        // Return it directly — wrapping it in another GEOGCS is invalid WKT1.
        return Ok(geogcs);
    }

    let (proj_name, params_list) = esri_projection_params(params);

    let mut wkt = format!("PROJCS[\"{}\",{},PROJECTION[\"{}\"]", projcs_name, geogcs, proj_name);
    for (k, v) in params_list {
        wkt.push_str(&format!(",PARAMETER[\"{}\",{:.12}]", k, v));
    }
    wkt.push_str(",UNIT[\"Meter\",1.0]]");
    Ok(wkt)
}

/// Generate an OGC-formatted WKT representation for an EPSG code.
///
/// Returns `Err(ProjectionError::UnsupportedProjection)` if the code is not
/// in the built-in registry.
pub fn to_ogc_wkt(code: u32) -> Result<String> {
    let crs = build_crs(code)?;
    let params = crs.projection.params();

    let datum = &crs.datum;
    let ellipsoid = &datum.ellipsoid;
    let inv_f = if ellipsoid.f.abs() < 1e-15 {
        0.0
    } else {
        1.0 / ellipsoid.f
    };

    let geogcs = format!(
        "GEOGCS[\"{}\",DATUM[\"{}\",SPHEROID[\"{}\",{:.3},{:.9}]],PRIMEM[\"Greenwich\",0.0],UNIT[\"degree\",0.0174532925199433]]",
        datum.name,
        datum.name,
        ellipsoid.name,
        ellipsoid.a,
        inv_f
    );

    let projcs_name = crs.name.replace('"', "'");

    if is_geocentric_epsg(code) {
        return Ok(format!(
            "GEOCCS[\"{}\",DATUM[\"{}\",SPHEROID[\"{}\",{:.3},{:.9}]],PRIMEM[\"Greenwich\",0.0],UNIT[\"metre\",1.0]]",
            projcs_name,
            datum.name,
            ellipsoid.name,
            ellipsoid.a,
            inv_f
        ));
    }

    if is_vertical_epsg(code) {
        let (axis_name, axis_dir) = vertical_axis_spec(code);
        let (unit_name, unit_scale) = vertical_wkt_unit(code, false);
        return Ok(format!(
            "VERT_CS[\"{}\",VERT_DATUM[\"{}\",2005],UNIT[\"{}\",{}],AXIS[\"{}\",{}]]",
            projcs_name,
            datum.name,
            unit_name,
            unit_scale,
            axis_name,
            axis_dir
        ));
    }

    if is_geographic_epsg(code) {
        // Geographic CRS: geogcs is already a complete GEOGCS[...] string.
        // Return it directly — wrapping it in another GEOGCS is invalid WKT1.
        return Ok(geogcs);
    }

    let (proj_name, params_list) = ogc_projection_params(params);

    let mut wkt = format!("PROJCS[\"{}\",{},PROJECTION[\"{}\"]", projcs_name, geogcs, proj_name);
    for (k, v) in params_list {
        wkt.push_str(&format!(",PARAMETER[\"{}\",{:.12}]", k, v));
    }
    wkt.push_str(",UNIT[\"metre\",1.0]]");
    Ok(wkt)
}

/// GeoTIFF projection info (GeoKeys) derived from an EPSG code.
#[derive(Debug, Clone)]
pub struct GeoTiffProjectionInfo {
    /// GeoTIFF GTModelTypeGeoKey (1 = Projected, 2 = Geographic, 3 = Geocentric/Vertical in this API).
    pub model_type: u16,
    /// GeoTIFF GTRasterTypeGeoKey (1 = PixelIsArea).
    pub raster_type: u16,
    /// GeoTIFF GeographicTypeGeoKey (EPSG code for geographic CRS).
    pub geographic_type: Option<u16>,
    /// GeoTIFF ProjectedCSTypeGeoKey (EPSG code for projected CRS).
    pub projected_cs_type: Option<u16>,
    /// GeoTIFF VerticalCSTypeGeoKey (EPSG code for vertical CRS).
    pub vertical_cs_type: Option<u16>,
    /// GeoTIFF ProjLinearUnitsGeoKey (EPSG linear units, e.g., 9001 = metre).
    pub linear_units: Option<u16>,
    /// GeoTIFF GeogAngularUnitsGeoKey (EPSG angular units, e.g., 9102 = degree).
    pub angular_units: Option<u16>,
}

/// Generate GeoTIFF projection info (GeoKeys) from an EPSG code.
///
/// Returns `Err(ProjectionError::UnsupportedProjection)` if the code is not
/// in the built-in registry.
pub fn to_geotiff_info(code: u32) -> Result<GeoTiffProjectionInfo> {
    // Ensure the code is supported and inspect CRS kind.
    let crs = build_crs(code)?;

    let info = epsg_info(code);
    let is_geog = is_geographic_epsg(code);

    let (model_type, geographic_type, projected_cs_type, vertical_cs_type) = if is_geocentric_epsg(code) {
        (3u16, None, None, None)
    } else if is_vertical_epsg(code) {
        (3u16, None, None, Some(code as u16))
    } else if is_geog {
        (2u16, Some(code as u16), None, None)
    } else {
        (1u16, None, Some(code as u16), None)
    };

    let mut linear_units = None;
    let mut angular_units = None;

    if let Some(i) = info {
        if i.unit.eq_ignore_ascii_case("metre") {
            linear_units = Some(9001);
        } else if i.unit.eq_ignore_ascii_case("US survey foot")
            || i.unit.eq_ignore_ascii_case("foot_us")
            || i.unit.eq_ignore_ascii_case("us_foot")
        {
            linear_units = Some(9003);
        } else if i.unit.eq_ignore_ascii_case("degree") {
            angular_units = Some(9102);
        }
    }

    if is_geog {
        angular_units = Some(9102);
    }

    if matches!(crs.projection.params().kind, ProjectionKind::Geocentric) {
        linear_units = Some(9001);
        angular_units = None;
    } else if matches!(crs.projection.params().kind, ProjectionKind::Vertical) {
        if linear_units.is_none() {
            linear_units = Some(9001);
        }
        angular_units = None;
    }

    Ok(GeoTiffProjectionInfo {
        model_type,
        raster_type: 1,
        geographic_type,
        projected_cs_type,
        vertical_cs_type,
        linear_units,
        angular_units,
    })
}

pub(crate) fn esri_projection_params(p: &ProjectionParams) -> (&'static str, Vec<(&'static str, f64)>) {
    use ProjectionKind::*;

    match p.kind {
        Geographic => (
            "Geographic",
            vec![],
        ),
        Geocentric => (
            "Geocentric",
            vec![],
        ),
        Geostationary { satellite_height, .. } => (
            "Geostationary_Satellite",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("satellite_height", satellite_height),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Vertical => (
            "Vertical",
            vec![],
        ),
        Mercator => (
            "Mercator",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WebMercator => (
            "Mercator_Auxiliary_Sphere",
            vec![
                ("central_meridian", p.lon0),
                ("standard_parallel_1", 0.0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TransverseMercator | TransverseMercatorSouthOrientated | Utm { .. } => (
            "Transverse_Mercator",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        LambertConformalConic { lat1, lat2 } => (
            "Lambert_Conformal_Conic",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2.unwrap_or(lat1)),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        AlbersEqualAreaConic { lat1, lat2 } => (
            "Albers",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        AzimuthalEquidistant => (
            "Azimuthal_Equidistant",
            vec![
                ("latitude_of_center", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TwoPointEquidistant {
            lon1,
            lat1,
            lon2,
            lat2,
        } => (
            "Two_Point_Equidistant",
            vec![
                ("longitude_of_1st_point", lon1),
                ("latitude_of_1st_point", lat1),
                ("longitude_of_2nd_point", lon2),
                ("latitude_of_2nd_point", lat2),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        LambertAzimuthalEqualArea => (
            "Lambert_Azimuthal_Equal_Area",
            vec![
                ("latitude_of_center", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Krovak => (
            "Krovak",
            vec![
                ("latitude_of_center", p.lat0),
                ("longitude_of_center", p.lon0),
                ("azimuth", 30.288_139_722_222_22),
                ("pseudo_standard_parallel_1", 78.5),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        HotineObliqueMercator {
            azimuth,
            rectified_grid_angle,
        } => (
            "Hotine_Oblique_Mercator_Azimuth_Center",
            vec![
                ("latitude_of_center", p.lat0),
                ("longitude_of_center", p.lon0),
                ("azimuth", azimuth),
                ("rectified_grid_angle", rectified_grid_angle.unwrap_or(azimuth)),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        CentralConic { lat1 } => (
            "Central_Conic",
            vec![
                ("standard_parallel_1", lat1),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Lagrange { lat1, w } => (
            "Lagrange",
            vec![
                ("latitude_of_origin", lat1),
                ("W", w),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Loximuthal { lat1 } => (
            "Loximuthal",
            vec![
                ("standard_parallel_1", lat1),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Euler { lat1, lat2 } => (
            "Euler",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Tissot { lat1, lat2 } => (
            "Tissot",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochI { lat1, lat2 } => (
            "Murdoch_I",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochII { lat1, lat2 } => (
            "Murdoch_II",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochIII { lat1, lat2 } => (
            "Murdoch_III",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PerspectiveConic { lat1, lat2 } => (
            "Perspective_Conic",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        VitkovskyI { lat1, lat2 } => (
            "Vitkovsky_I",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        ToblerMercator => (
            "Tobler_Mercator",
            vec![
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelII => (
            "Winkel_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        KavrayskiyV => (
            "Kavrayskiy_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Stereographic => (
            "Stereographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        ObliqueStereographic => (
            "Oblique_Stereographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PolarStereographic { north, lat_ts } => {
            let method = if lat_ts.is_none() {
                "Polar_Stereographic_Variant_A"
            } else if north {
                "Stereographic_North_Pole"
            } else {
                "Stereographic_South_Pole"
            };
            let mut v = vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ];
            if let Some(phi) = lat_ts {
                v.push(("standard_parallel_1", phi));
            } else {
                v.push(("scale_factor", p.scale));
            }
            (method, v)
        },
        Orthographic => (
            "Orthographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Sinusoidal => (
            "Sinusoidal",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mollweide => (
            "Mollweide",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MbtFps => (
            "Mcbryde_Thomas_Flat_Pole_Sine",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MbtS => (
            "Mcbryde_Thomas_Flat_Polar_Sine",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mbtfpp => (
            "Mcbryde_Thomas_Flat_Polar_Parabolic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mbtfpq => (
            "Mcbryde_Thomas_Flat_Polar_Quartic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Nell => (
            "Nell",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EqualEarth => (
            "Equal_Earth",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        CylindricalEqualArea { lat_ts } => (
            "Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", lat_ts),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Equirectangular { lat_ts } => (
            "Plate_Carree",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("standard_parallel_1", lat_ts),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Robinson => (
            "Robinson",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Gnomonic => (
            "Gnomonic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Aitoff => (
            "Aitoff",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        VanDerGrinten => (
            "Van_der_Grinten_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelTripel => (
            "Winkel_Tripel",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Hammer => (
            "Hammer_Aitoff",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Hatano => (
            "Hatano_Asymmetrical_Equal_Area",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertI => (
            "Eckert_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertII => (
            "Eckert_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertIII => (
            "Eckert_III",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertIV => (
            "Eckert_IV",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertV => (
            "Eckert_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MillerCylindrical => (
            "Miller_Cylindrical",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        GallStereographic => (
            "Gall_Stereographic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        GallPeters => (
            "Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 45.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Behrmann => (
            "Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 30.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        HoboDyer => (
            "Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 37.5),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerI => (
            "Wagner_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerII => (
            "Wagner_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerIII => (
            "Wagner_III",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerIV => (
            "Wagner_IV",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerV => (
            "Wagner_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NaturalEarth => (
            "Natural_Earth",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NaturalEarthII => (
            "Natural_Earth_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerVI => (
            "Wagner_VI",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertVI => (
            "Eckert_VI",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TransverseCylindricalEqualArea => (
            "Transverse_Cylindrical_Equal_Area",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Polyconic => (
            "Polyconic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Cassini => (
            "Cassini",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Bonne | BonneSouthOrientated => (
            "Bonne",
            vec![
                ("standard_parallel_1", 45.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Craster => (
            "Craster_Parabolic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP4p => (
            "Putnins_P4p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Fahey => (
            "Fahey",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Times => (
            "Times",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Patterson => (
            "Patterson",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP3 => (
            "Putnins_P3",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP3p => (
            "Putnins_P3p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP5 => (
            "Putnins_P5",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP5p => (
            "Putnins_P5p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP1 => (
            "Putnins_P1",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP2 => (
            "Putnins_P2",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP6 => (
            "Putnins_P6",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP6p => (
            "Putnins_P6p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        QuarticAuthalic => (
            "Quartic_Authalic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Foucaut => (
            "Foucaut",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelI => (
            "Winkel_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WerenskioldI => (
            "Werenskiold_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Collignon => (
            "Collignon",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NellHammer => (
            "Nell_Hammer",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        KavrayskiyVII => (
            "Kavrayskiy_VII",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
    }
}

fn is_geographic_epsg(code: u32) -> bool {
    matches!(
        code,
        4326 | 4269 | 4267 | 4258 | 4230 | 4617
            | 4283 | 4148 | 4152 | 4167 | 4189 | 4619
            | 4681 | 4483 | 4624 | 4284 | 4322 | 6318 | 4615
            | 4490 | 4674 | 7843 | 7844 | 4610 | 4612
    )
}

fn is_geocentric_epsg(code: u32) -> bool {
    matches!(code, 7842)
}

fn is_vertical_epsg(code: u32) -> bool {
    matches!(code, 3855 | 5701 | 5702 | 5703 | 5714 | 5715 | 5773 | 7841 | 8228)
}

fn vertical_axis_spec(code: u32) -> (&'static str, &'static str) {
    if matches!(code, 5715) {
        ("gravity-related depth", "DOWN")
    } else {
        ("gravity-related height", "UP")
    }
}

fn vertical_wkt_unit(code: u32, esri: bool) -> (&'static str, f64) {
    let unit = get_info(code).map(|i| i.unit).unwrap_or("metre");

    if unit.eq_ignore_ascii_case("US survey foot") {
        if esri {
            ("Foot_US", 1200.0 / 3937.0)
        } else {
            ("US survey foot", 1200.0 / 3937.0)
        }
    } else if esri {
        ("Meter", 1.0)
    } else {
        ("metre", 1.0)
    }
}

pub(crate) fn ogc_projection_params(p: &ProjectionParams) -> (&'static str, Vec<(&'static str, f64)>) {
    use ProjectionKind::*;

    match p.kind {
        Geographic => (
            "Geographic",
            vec![],
        ),
        Geocentric => (
            "Geocentric",
            vec![],
        ),
        Geostationary { satellite_height, .. } => (
            "Geostationary_Satellite",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("satellite_height", satellite_height),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Vertical => (
            "Vertical",
            vec![],
        ),
        Mercator => (
            "Mercator_1SP",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WebMercator => (
            "Mercator_1SP",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", 0.0),
                ("scale_factor", 1.0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TransverseMercator | TransverseMercatorSouthOrientated | Utm { .. } => (
            "Transverse_Mercator",
            vec![
                ("central_meridian", p.lon0),
                ("latitude_of_origin", p.lat0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        LambertConformalConic { lat1, lat2 } => (
            "Lambert_Conformal_Conic_2SP",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2.unwrap_or(lat1)),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        AlbersEqualAreaConic { lat1, lat2 } => (
            "Albers_Conic_Equal_Area",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        AzimuthalEquidistant => (
            "Azimuthal_Equidistant",
            vec![
                ("latitude_of_center", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TwoPointEquidistant {
            lon1,
            lat1,
            lon2,
            lat2,
        } => (
            "Two_Point_Equidistant",
            vec![
                ("longitude_of_1st_point", lon1),
                ("latitude_of_1st_point", lat1),
                ("longitude_of_2nd_point", lon2),
                ("latitude_of_2nd_point", lat2),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        LambertAzimuthalEqualArea => (
            "Lambert_Azimuthal_Equal_Area",
            vec![
                ("latitude_of_center", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Krovak => (
            "Krovak",
            vec![
                ("latitude_of_center", p.lat0),
                ("longitude_of_center", p.lon0),
                ("azimuth", 30.288_139_722_222_22),
                ("pseudo_standard_parallel_1", 78.5),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        HotineObliqueMercator {
            azimuth,
            rectified_grid_angle,
        } => (
            "Hotine_Oblique_Mercator_Azimuth_Center",
            vec![
                ("latitude_of_center", p.lat0),
                ("longitude_of_center", p.lon0),
                ("azimuth", azimuth),
                ("rectified_grid_angle", rectified_grid_angle.unwrap_or(azimuth)),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        CentralConic { lat1 } => (
            "Central_Conic",
            vec![
                ("standard_parallel_1", lat1),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Lagrange { lat1, w } => (
            "Lagrange",
            vec![
                ("latitude_of_origin", lat1),
                ("W", w),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Loximuthal { lat1 } => (
            "Loximuthal",
            vec![
                ("standard_parallel_1", lat1),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Euler { lat1, lat2 } => (
            "Euler",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Tissot { lat1, lat2 } => (
            "Tissot",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochI { lat1, lat2 } => (
            "Murdoch_I",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochII { lat1, lat2 } => (
            "Murdoch_II",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MurdochIII { lat1, lat2 } => (
            "Murdoch_III",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PerspectiveConic { lat1, lat2 } => (
            "Perspective_Conic",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        VitkovskyI { lat1, lat2 } => (
            "Vitkovsky_I",
            vec![
                ("standard_parallel_1", lat1),
                ("standard_parallel_2", lat2),
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        ToblerMercator => (
            "Tobler_Mercator",
            vec![
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelII => (
            "Winkel_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        KavrayskiyV => (
            "Kavrayskiy_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Stereographic => (
            "Stereographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        ObliqueStereographic => (
            "Oblique_Stereographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PolarStereographic { north, lat_ts } => {
            let method = if lat_ts.is_none() {
                "Polar_Stereographic_Variant_A"
            } else if north {
                "Stereographic_North_Pole"
            } else {
                "Stereographic_South_Pole"
            };
            let mut v = vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ];
            if let Some(phi) = lat_ts {
                v.push(("standard_parallel_1", phi));
            } else {
                v.push(("scale_factor", p.scale));
            }
            (method, v)
        },
        Orthographic => (
            "Orthographic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Sinusoidal => (
            "Sinusoidal",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mollweide => (
            "Mollweide",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MbtFps => (
            "Mcbryde_Thomas_Flat_Pole_Sine",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MbtS => (
            "Mcbryde_Thomas_Flat_Polar_Sine",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mbtfpp => (
            "Mcbryde_Thomas_Flat_Polar_Parabolic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Mbtfpq => (
            "Mcbryde_Thomas_Flat_Polar_Quartic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Nell => (
            "Nell",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EqualEarth => (
            "Equal_Earth",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        CylindricalEqualArea { lat_ts } => (
            "Lambert_Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", lat_ts),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Equirectangular { lat_ts } => (
            "Equirectangular",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("standard_parallel_1", lat_ts),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Robinson => (
            "Robinson",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Gnomonic => (
            "Gnomonic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Aitoff => (
            "Aitoff",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        VanDerGrinten => (
            "Van_der_Grinten_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelTripel => (
            "Winkel_Tripel",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Hammer => (
            "Hammer_Aitoff",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Hatano => (
            "Hatano_Asymmetrical_Equal_Area",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertI => (
            "Eckert_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertII => (
            "Eckert_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertIII => (
            "Eckert_III",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertIV => (
            "Eckert_IV",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertV => (
            "Eckert_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        MillerCylindrical => (
            "Miller_Cylindrical",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        GallStereographic => (
            "Gall_Stereographic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        GallPeters => (
            "Lambert_Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 45.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Behrmann => (
            "Lambert_Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 30.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        HoboDyer => (
            "Lambert_Cylindrical_Equal_Area",
            vec![
                ("standard_parallel_1", 37.5),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerI => (
            "Wagner_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerII => (
            "Wagner_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerIII => (
            "Wagner_III",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerIV => (
            "Wagner_IV",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerV => (
            "Wagner_V",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NaturalEarth => (
            "Natural_Earth",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NaturalEarthII => (
            "Natural_Earth_II",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WagnerVI => (
            "Wagner_VI",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        EckertVI => (
            "Eckert_VI",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        TransverseCylindricalEqualArea => (
            "Transverse_Cylindrical_Equal_Area",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("scale_factor", p.scale),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Polyconic => (
            "Polyconic",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Cassini => (
            "Cassini",
            vec![
                ("latitude_of_origin", p.lat0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Bonne | BonneSouthOrientated => (
            "Bonne",
            vec![
                ("standard_parallel_1", 45.0),
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Craster => (
            "Craster_Parabolic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP4p => (
            "Putnins_P4p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Fahey => (
            "Fahey",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Times => (
            "Times",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Patterson => (
            "Patterson",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP3 => (
            "Putnins_P3",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP3p => (
            "Putnins_P3p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP5 => (
            "Putnins_P5",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP5p => (
            "Putnins_P5p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP1 => (
            "Putnins_P1",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP2 => (
            "Putnins_P2",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP6 => (
            "Putnins_P6",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        PutninsP6p => (
            "Putnins_P6p",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        QuarticAuthalic => (
            "Quartic_Authalic",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Foucaut => (
            "Foucaut",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WinkelI => (
            "Winkel_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        WerenskioldI => (
            "Werenskiold_I",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        Collignon => (
            "Collignon",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        NellHammer => (
            "Nell_Hammer",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
        KavrayskiyVII => (
            "Kavrayskiy_VII",
            vec![
                ("central_meridian", p.lon0),
                ("false_easting", p.false_easting),
                ("false_northing", p.false_northing),
            ],
        ),
    }
}

// ─── named registry entries ────────────────────────────────────────────────
// Each entry: (code, name, area_of_use, unit)
static NAMED_ENTRIES: &[(u32, &str, &str, &str)] = &[
    (3855,  "EGM2008 height",                      "World",                      "metre"),
    (5701,  "ODN height",                          "United Kingdom",             "metre"),
    (5702,  "NGVD 29 height",                      "United States (CONUS)",      "US survey foot"),
    (5703,  "NAVD88 height",                       "United States (CONUS)",      "metre"),
    (5714,  "MSL height",                          "World",                      "metre"),
    (5715,  "MSL depth",                           "World",                      "metre"),
    (5773,  "EGM96 height",                        "World",                      "metre"),
    (7841,  "GDA2020 height",                      "Australia",                  "metre"),
    (7842,  "GDA2020",                             "Australia",                  "metre"),
    (8228,  "NAVD88 height",                       "United States (CONUS)",      "US survey foot"),
    (4326,  "WGS 84",                               "World",                       "degree"),
    (4269,  "NAD83",                                "North America",               "degree"),
    (4267,  "NAD27",                                "North America",               "degree"),
    (4617,  "NAD83(CSRS)",                          "Canada",                      "degree"),
    (4954,  "NAD83(CSRS)",                          "Canada",                      "degree"),
    (4955,  "NAD83(CSRS)",                          "Canada",                      "degree"),
    (8230,  "NAD83(CSRS96)",                        "Canada",                      "degree"),
    (8231,  "NAD83(CSRS96)",                        "Canada",                      "degree"),
    (8232,  "NAD83(CSRS96)",                        "Canada",                      "degree"),
    (8233,  "NAD83(CSRS)v2",                        "Canada",                      "degree"),
    (8235,  "NAD83(CSRS)v2",                        "Canada",                      "degree"),
    (8237,  "NAD83(CSRS)v2",                        "Canada",                      "degree"),
    (8238,  "NAD83(CSRS)v3",                        "Canada",                      "degree"),
    (8239,  "NAD83(CSRS)v3",                        "Canada",                      "degree"),
    (8240,  "NAD83(CSRS)v3",                        "Canada",                      "degree"),
    (8242,  "NAD83(CSRS)v4",                        "Canada",                      "degree"),
    (8244,  "NAD83(CSRS)v4",                        "Canada",                      "degree"),
    (8246,  "NAD83(CSRS)v4",                        "Canada",                      "degree"),
    (8247,  "NAD83(CSRS)v5",                        "Canada",                      "degree"),
    (8248,  "NAD83(CSRS)v5",                        "Canada",                      "degree"),
    (8249,  "NAD83(CSRS)v5",                        "Canada",                      "degree"),
    (8250,  "NAD83(CSRS)v6",                        "Canada",                      "degree"),
    (8251,  "NAD83(CSRS)v6",                        "Canada",                      "degree"),
    (8252,  "NAD83(CSRS)v6",                        "Canada",                      "degree"),
    (8253,  "NAD83(CSRS)v7",                        "Canada",                      "degree"),
    (8254,  "NAD83(CSRS)v7",                        "Canada",                      "degree"),
    (8255,  "NAD83(CSRS)v7",                        "Canada",                      "degree"),
    (10413, "NAD83(CSRS)v8",                        "Canada",                      "degree"),
    (10414, "NAD83(CSRS)v8",                        "Canada",                      "degree"),
    (4258,  "ETRS89",                               "Europe",                      "degree"),
    (4230,  "ED50",                                 "Europe",                      "degree"),
    (4490,  "China Geodetic Coordinate System 2000", "China",                     "degree"),
    (4491,  "CGCS2000 / Gauss-Kruger zone 13",      "China",                      "metre"),
    (4492,  "CGCS2000 / Gauss-Kruger zone 14",      "China",                      "metre"),
    (4493,  "CGCS2000 / Gauss-Kruger zone 15",      "China",                      "metre"),
    (4494,  "CGCS2000 / Gauss-Kruger zone 16",      "China",                      "metre"),
    (4495,  "CGCS2000 / Gauss-Kruger zone 17",      "China",                      "metre"),
    (4496,  "CGCS2000 / Gauss-Kruger zone 18",      "China",                      "metre"),
    (4497,  "CGCS2000 / Gauss-Kruger zone 19",      "China",                      "metre"),
    (4498,  "CGCS2000 / Gauss-Kruger zone 20",      "China",                      "metre"),
    (4499,  "CGCS2000 / Gauss-Kruger zone 21",      "China",                      "metre"),
    (4500,  "CGCS2000 / Gauss-Kruger zone 22",      "China",                      "metre"),
    (4501,  "CGCS2000 / Gauss-Kruger zone 23",      "China",                      "metre"),
    (4502,  "CGCS2000 / Gauss-Kruger CM 75E",       "China",                      "metre"),
    (4503,  "CGCS2000 / Gauss-Kruger CM 81E",       "China",                      "metre"),
    (4504,  "CGCS2000 / Gauss-Kruger CM 87E",       "China",                      "metre"),
    (4505,  "CGCS2000 / Gauss-Kruger CM 93E",       "China",                      "metre"),
    (4506,  "CGCS2000 / Gauss-Kruger CM 99E",       "China",                      "metre"),
    (4507,  "CGCS2000 / Gauss-Kruger CM 105E",      "China",                      "metre"),
    (4508,  "CGCS2000 / Gauss-Kruger CM 111E",      "China",                      "metre"),
    (4509,  "CGCS2000 / Gauss-Kruger CM 117E",      "China",                      "metre"),
    (4510,  "CGCS2000 / Gauss-Kruger CM 123E",      "China",                      "metre"),
    (4511,  "CGCS2000 / Gauss-Kruger CM 129E",      "China",                      "metre"),
    (4512,  "CGCS2000 / Gauss-Kruger CM 135E",      "China",                      "metre"),
    (4513,  "CGCS2000 / 3-degree Gauss-Kruger zone 25", "China",                  "metre"),
    (4514,  "CGCS2000 / 3-degree Gauss-Kruger zone 26", "China",                  "metre"),
    (4515,  "CGCS2000 / 3-degree Gauss-Kruger zone 27", "China",                  "metre"),
    (4516,  "CGCS2000 / 3-degree Gauss-Kruger zone 28", "China",                  "metre"),
    (4517,  "CGCS2000 / 3-degree Gauss-Kruger zone 29", "China",                  "metre"),
    (4518,  "CGCS2000 / 3-degree Gauss-Kruger zone 30", "China",                  "metre"),
    (4519,  "CGCS2000 / 3-degree Gauss-Kruger zone 31", "China",                  "metre"),
    (4520,  "CGCS2000 / 3-degree Gauss-Kruger zone 32", "China",                  "metre"),
    (4521,  "CGCS2000 / 3-degree Gauss-Kruger zone 33", "China",                  "metre"),
    (4522,  "CGCS2000 / 3-degree Gauss-Kruger zone 34", "China",                  "metre"),
    (4523,  "CGCS2000 / 3-degree Gauss-Kruger zone 35", "China",                  "metre"),
    (4524,  "CGCS2000 / 3-degree Gauss-Kruger zone 36", "China",                  "metre"),
    (4525,  "CGCS2000 / 3-degree Gauss-Kruger zone 37", "China",                  "metre"),
    (4526,  "CGCS2000 / 3-degree Gauss-Kruger zone 38", "China",                  "metre"),
    (4527,  "CGCS2000 / 3-degree Gauss-Kruger zone 39", "China",                  "metre"),
    (4528,  "CGCS2000 / 3-degree Gauss-Kruger zone 40", "China",                  "metre"),
    (4529,  "CGCS2000 / 3-degree Gauss-Kruger zone 41", "China",                  "metre"),
    (4530,  "CGCS2000 / 3-degree Gauss-Kruger zone 42", "China",                  "metre"),
    (4531,  "CGCS2000 / 3-degree Gauss-Kruger zone 43", "China",                  "metre"),
    (4532,  "CGCS2000 / 3-degree Gauss-Kruger zone 44", "China",                  "metre"),
    (4533,  "CGCS2000 / 3-degree Gauss-Kruger zone 45", "China",                  "metre"),
    (4534,  "CGCS2000 / 3-degree Gauss-Kruger CM 75E", "China",                   "metre"),
    (4535,  "CGCS2000 / 3-degree Gauss-Kruger CM 78E", "China",                   "metre"),
    (4536,  "CGCS2000 / 3-degree Gauss-Kruger CM 81E", "China",                   "metre"),
    (4537,  "CGCS2000 / 3-degree Gauss-Kruger CM 84E", "China",                   "metre"),
    (4538,  "CGCS2000 / 3-degree Gauss-Kruger CM 87E", "China",                   "metre"),
    (4539,  "CGCS2000 / 3-degree Gauss-Kruger CM 90E", "China",                   "metre"),
    (4540,  "CGCS2000 / 3-degree Gauss-Kruger CM 93E", "China",                   "metre"),
    (4541,  "CGCS2000 / 3-degree Gauss-Kruger CM 96E", "China",                   "metre"),
    (4542,  "CGCS2000 / 3-degree Gauss-Kruger CM 99E", "China",                   "metre"),
    (4543,  "CGCS2000 / 3-degree Gauss-Kruger CM 102E", "China",                  "metre"),
    (4544,  "CGCS2000 / 3-degree Gauss-Kruger CM 105E", "China",                  "metre"),
    (4545,  "CGCS2000 / 3-degree Gauss-Kruger CM 108E", "China",                  "metre"),
    (4546,  "CGCS2000 / 3-degree Gauss-Kruger CM 111E", "China",                  "metre"),
    (4547,  "CGCS2000 / 3-degree Gauss-Kruger CM 114E", "China",                  "metre"),
    (4548,  "CGCS2000 / 3-degree Gauss-Kruger CM 117E", "China",                  "metre"),
    (4549,  "CGCS2000 / 3-degree Gauss-Kruger CM 120E", "China",                  "metre"),
    (4550,  "CGCS2000 / 3-degree Gauss-Kruger CM 123E", "China",                  "metre"),
    (4551,  "CGCS2000 / 3-degree Gauss-Kruger CM 126E", "China",                  "metre"),
    (4552,  "CGCS2000 / 3-degree Gauss-Kruger CM 129E", "China",                  "metre"),
    (4553,  "CGCS2000 / 3-degree Gauss-Kruger CM 132E", "China",                  "metre"),
    (4554,  "CGCS2000 / 3-degree Gauss-Kruger CM 135E", "China",                  "metre"),
    (4568,  "New Beijing / Gauss-Kruger zone 13",      "China",                   "metre"),
    (4569,  "New Beijing / Gauss-Kruger zone 14",      "China",                   "metre"),
    (4570,  "New Beijing / Gauss-Kruger zone 15",      "China",                   "metre"),
    (4571,  "New Beijing / Gauss-Kruger zone 16",      "China",                   "metre"),
    (4572,  "New Beijing / Gauss-Kruger zone 17",      "China",                   "metre"),
    (4573,  "New Beijing / Gauss-Kruger zone 18",      "China",                   "metre"),
    (4574,  "New Beijing / Gauss-Kruger zone 19",      "China",                   "metre"),
    (4575,  "New Beijing / Gauss-Kruger zone 20",      "China",                   "metre"),
    (4576,  "New Beijing / Gauss-Kruger zone 21",      "China",                   "metre"),
    (4577,  "New Beijing / Gauss-Kruger zone 22",      "China",                   "metre"),
    (4578,  "New Beijing / Gauss-Kruger zone 23",      "China",                   "metre"),
    (4579,  "New Beijing / Gauss-Kruger CM 75E",       "China",                   "metre"),
    (4580,  "New Beijing / Gauss-Kruger CM 81E",       "China",                   "metre"),
    (4581,  "New Beijing / Gauss-Kruger CM 87E",       "China",                   "metre"),
    (4582,  "New Beijing / Gauss-Kruger CM 93E",       "China",                   "metre"),
    (4583,  "New Beijing / Gauss-Kruger CM 99E",       "China",                   "metre"),
    (4584,  "New Beijing / Gauss-Kruger CM 105E",      "China",                   "metre"),
    (4585,  "New Beijing / Gauss-Kruger CM 111E",      "China",                   "metre"),
    (4586,  "New Beijing / Gauss-Kruger CM 117E",      "China",                   "metre"),
    (4587,  "New Beijing / Gauss-Kruger CM 123E",      "China",                   "metre"),
    (4588,  "New Beijing / Gauss-Kruger CM 129E",      "China",                   "metre"),
    (4589,  "New Beijing / Gauss-Kruger CM 135E",      "China",                   "metre"),
    (4601,  "Antigua 1943",                            "Antigua and Barbuda",     "degree"),
    (4602,  "Dominica 1945",                           "Dominica",                "degree"),
    (4603,  "Grenada 1953",                            "Grenada",                 "degree"),
    (4604,  "Montserrat 1958",                         "Montserrat",              "degree"),
    (4605,  "St. Kitts 1955",                          "St. Kitts and Nevis",     "degree"),
    (4610,  "Xian 1980",                               "China",                   "degree"),
    (4612,  "JGD2000",                                 "Japan",                   "degree"),
    (4652,  "New Beijing / 3-degree Gauss-Kruger zone 25", "China",               "metre"),
    (4653,  "New Beijing / 3-degree Gauss-Kruger zone 26", "China",               "metre"),
    (4654,  "New Beijing / 3-degree Gauss-Kruger zone 27", "China",               "metre"),
    (4655,  "New Beijing / 3-degree Gauss-Kruger zone 28", "China",               "metre"),
    (4656,  "New Beijing / 3-degree Gauss-Kruger zone 29", "China",               "metre"),
    (4674,  "SIRGAS 2000",                          "Latin America",              "degree"),
    (4766,  "New Beijing / 3-degree Gauss-Kruger zone 30", "China",               "metre"),
    (4767,  "New Beijing / 3-degree Gauss-Kruger zone 31", "China",               "metre"),
    (4768,  "New Beijing / 3-degree Gauss-Kruger zone 32", "China",               "metre"),
    (4769,  "New Beijing / 3-degree Gauss-Kruger zone 33", "China",               "metre"),
    (4770,  "New Beijing / 3-degree Gauss-Kruger zone 34", "China",               "metre"),
    (4771,  "New Beijing / 3-degree Gauss-Kruger zone 35", "China",               "metre"),
    (4772,  "New Beijing / 3-degree Gauss-Kruger zone 36", "China",               "metre"),
    (4773,  "New Beijing / 3-degree Gauss-Kruger zone 37", "China",               "metre"),
    (4774,  "New Beijing / 3-degree Gauss-Kruger zone 38", "China",               "metre"),
    (4775,  "New Beijing / 3-degree Gauss-Kruger zone 39", "China",               "metre"),
    (4776,  "New Beijing / 3-degree Gauss-Kruger zone 40", "China",               "metre"),
    (4777,  "New Beijing / 3-degree Gauss-Kruger zone 41", "China",               "metre"),
    (4778,  "New Beijing / 3-degree Gauss-Kruger zone 42", "China",               "metre"),
    (4779,  "New Beijing / 3-degree Gauss-Kruger zone 43", "China",               "metre"),
    (4780,  "New Beijing / 3-degree Gauss-Kruger zone 44", "China",               "metre"),
    (4781,  "New Beijing / 3-degree Gauss-Kruger zone 45", "China",               "metre"),
    (4782,  "New Beijing / 3-degree Gauss-Kruger CM 75E",  "China",               "metre"),
    (4783,  "New Beijing / 3-degree Gauss-Kruger CM 78E",  "China",               "metre"),
    (4784,  "New Beijing / 3-degree Gauss-Kruger CM 81E",  "China",               "metre"),
    (4785,  "New Beijing / 3-degree Gauss-Kruger CM 84E",  "China",               "metre"),
    (4786,  "New Beijing / 3-degree Gauss-Kruger CM 87E",  "China",               "metre"),
    (4787,  "New Beijing / 3-degree Gauss-Kruger CM 90E",  "China",               "metre"),
    (4788,  "New Beijing / 3-degree Gauss-Kruger CM 93E",  "China",               "metre"),
    (4789,  "New Beijing / 3-degree Gauss-Kruger CM 96E",  "China",               "metre"),
    (4790,  "New Beijing / 3-degree Gauss-Kruger CM 99E",  "China",               "metre"),
    (4791,  "New Beijing / 3-degree Gauss-Kruger CM 102E", "China",               "metre"),
    (4792,  "New Beijing / 3-degree Gauss-Kruger CM 105E", "China",               "metre"),
    (4793,  "New Beijing / 3-degree Gauss-Kruger CM 108E", "China",               "metre"),
    (4794,  "New Beijing / 3-degree Gauss-Kruger CM 111E", "China",               "metre"),
    (4795,  "New Beijing / 3-degree Gauss-Kruger CM 114E", "China",               "metre"),
    (4796,  "New Beijing / 3-degree Gauss-Kruger CM 117E", "China",               "metre"),
    (4797,  "New Beijing / 3-degree Gauss-Kruger CM 120E", "China",               "metre"),
    (4798,  "New Beijing / 3-degree Gauss-Kruger CM 123E", "China",               "metre"),
    (4799,  "New Beijing / 3-degree Gauss-Kruger CM 126E", "China",               "metre"),
    (4800,  "New Beijing / 3-degree Gauss-Kruger CM 129E", "China",               "metre"),
    (4812,  "New Beijing / 3-degree Gauss-Kruger CM 132E", "China",               "metre"),
    (4822,  "New Beijing / 3-degree Gauss-Kruger CM 135E", "China",               "metre"),
    (4855,  "ETRS89-NOR [EUREF89] / NTM zone 5",           "Norway",              "metre"),
    (4856,  "ETRS89-NOR [EUREF89] / NTM zone 6",           "Norway",              "metre"),
    (4857,  "ETRS89-NOR [EUREF89] / NTM zone 7",           "Norway",              "metre"),
    (4858,  "ETRS89-NOR [EUREF89] / NTM zone 8",           "Norway",              "metre"),
    (4859,  "ETRS89-NOR [EUREF89] / NTM zone 9",           "Norway",              "metre"),
    (4860,  "ETRS89-NOR [EUREF89] / NTM zone 10",          "Norway",              "metre"),
    (4861,  "ETRS89-NOR [EUREF89] / NTM zone 11",          "Norway",              "metre"),
    (4862,  "ETRS89-NOR [EUREF89] / NTM zone 12",          "Norway",              "metre"),
    (4863,  "ETRS89-NOR [EUREF89] / NTM zone 13",          "Norway",              "metre"),
    (4864,  "ETRS89-NOR [EUREF89] / NTM zone 14",          "Norway",              "metre"),
    (4865,  "ETRS89-NOR [EUREF89] / NTM zone 15",          "Norway",              "metre"),
    (4866,  "ETRS89-NOR [EUREF89] / NTM zone 16",          "Norway",              "metre"),
    (4867,  "ETRS89-NOR [EUREF89] / NTM zone 17",          "Norway",              "metre"),
    (5105,  "ETRS89-NOR [EUREF89] / NTM zone 5",           "Norway",              "metre"),
    (5106,  "ETRS89-NOR [EUREF89] / NTM zone 6",           "Norway",              "metre"),
    (5107,  "ETRS89-NOR [EUREF89] / NTM zone 7",           "Norway",              "metre"),
    (5108,  "ETRS89-NOR [EUREF89] / NTM zone 8",           "Norway",              "metre"),
    (5109,  "ETRS89-NOR [EUREF89] / NTM zone 9",           "Norway",              "metre"),
    (5110,  "ETRS89-NOR [EUREF89] / NTM zone 10",          "Norway",              "metre"),
    (5111,  "ETRS89-NOR [EUREF89] / NTM zone 11",          "Norway",              "metre"),
    (5112,  "ETRS89-NOR [EUREF89] / NTM zone 12",          "Norway",              "metre"),
    (5113,  "ETRS89-NOR [EUREF89] / NTM zone 13",          "Norway",              "metre"),
    (5114,  "ETRS89-NOR [EUREF89] / NTM zone 14",          "Norway",              "metre"),
    (5115,  "ETRS89-NOR [EUREF89] / NTM zone 15",          "Norway",              "metre"),
    (5116,  "ETRS89-NOR [EUREF89] / NTM zone 16",          "Norway",              "metre"),
    (5117,  "ETRS89-NOR [EUREF89] / NTM zone 17",          "Norway",              "metre"),
    (5118,  "ETRS89-NOR [EUREF89] / NTM zone 18",          "Norway",              "metre"),
    (5119,  "ETRS89-NOR [EUREF89] / NTM zone 19",          "Norway",              "metre"),
    (5120,  "ETRS89-NOR [EUREF89] / NTM zone 20",          "Norway",              "metre"),
    (5121,  "ETRS89-NOR [EUREF89] / NTM zone 21",          "Norway",              "metre"),
    (5122,  "ETRS89-NOR [EUREF89] / NTM zone 22",          "Norway",              "metre"),
    (5123,  "ETRS89-NOR [EUREF89] / NTM zone 23",          "Norway",              "metre"),
    (5124,  "ETRS89-NOR [EUREF89] / NTM zone 24",          "Norway",              "metre"),
    (5125,  "ETRS89-NOR [EUREF89] / NTM zone 25",          "Norway",              "metre"),
    (5126,  "ETRS89-NOR [EUREF89] / NTM zone 26",          "Norway",              "metre"),
    (5127,  "ETRS89-NOR [EUREF89] / NTM zone 27",          "Norway",              "metre"),
    (5128,  "ETRS89-NOR [EUREF89] / NTM zone 28",          "Norway",              "metre"),
    (5129,  "ETRS89-NOR [EUREF89] / NTM zone 29",          "Norway",              "metre"),
    (7844,  "GDA2020",                               "Australia",                  "degree"),
    (3857,  "WGS 84 / Pseudo-Mercator",             "World (web maps)",            "metre"),
    (3395,  "WGS 84 / World Mercator",              "World",                       "metre"),
    (2163,  "US National Atlas Equal Area",         "United States",               "metre"),
    (3400,  "NAD83 / Alberta 10-TM (Forest)",       "Canada - Alberta",            "metre"),
    (3401,  "NAD83 / Alberta 10-TM (Resource)",     "Canada - Alberta",            "metre"),
    (3402,  "NAD83(CSRS) / Alberta 10-TM (Forest)", "Canada - Alberta",            "metre"),
    (3403,  "NAD83(CSRS) / Alberta 10-TM (Resource)", "Canada - Alberta",         "metre"),
    (3405,  "VN-2000 / UTM zone 48N",               "Vietnam",                     "metre"),
    (3406,  "VN-2000 / UTM zone 49N",               "Vietnam",                     "metre"),
    (3408,  "NSIDC EASE-Grid North",                "Northern hemisphere",         "metre"),
    (3409,  "NSIDC EASE-Grid South",                "Southern hemisphere",         "metre"),
    (4087,  "WGS 84 / World Equidistant Cylindrical", "World",                    "metre"),
    (3410,  "NSIDC EASE-Grid Global",               "World between 86°S and 86°N", "metre"),
    (5070,  "NAD83 / Conus Albers",                 "CONUS, USA",                  "metre"),
    (32662, "WGS 84 / Plate Carree",                "World",                       "metre"),
    (32661, "WGS 84 / UPS North (N,E)",             "Northern hemisphere >60°N",   "metre"),
    (32761, "WGS 84 / UPS South (N,E)",             "Southern hemisphere <60°S",   "metre"),
    // ETRS89 pan-European
    (3034,  "ETRS89 / LCC Europe",                  "Europe",                      "metre"),
    (3035,  "ETRS89 / LAEA Europe",                 "Europe",                      "metre"),
    (3031,  "WGS 84 / Antarctic Polar Stereographic", "Antarctica",               "metre"),
    (3032,  "WGS 84 / Australian Antarctic Polar Stereographic", "Antarctica",    "metre"),
    (3413,  "WGS 84 / NSIDC Sea Ice Polar Stereographic North", "Arctic",         "metre"),
    (3976,  "WGS 84 / NSIDC Sea Ice Polar Stereographic South", "Antarctic",      "metre"),
    (3996,  "WGS 84 / IBCAO Polar Stereographic",   "Arctic",                      "metre"),
    (3995,  "WGS 84 / Arctic Polar Stereographic", "Arctic",                      "metre"),
    (6931,  "WGS 84 / NSIDC EASE-Grid 2.0 North",   "Northern hemisphere",        "metre"),
    (6932,  "WGS 84 / NSIDC EASE-Grid 2.0 South",   "Southern hemisphere",        "metre"),
    (6933,  "WGS 84 / NSIDC EASE-Grid 2.0 Global",  "World between 86°S and 86°N", "metre"),
    (8857,  "WGS 84 / Equal Earth Greenwich",       "World",                      "metre"),
    (3577,  "GDA94 / Australian Albers",            "Australia",                  "metre"),
    (3578,  "NAD83 / Yukon Albers",                 "Canada - Yukon",             "metre"),
    (3579,  "NAD83(CSRS) / Yukon Albers",           "Canada - Yukon",             "metre"),
    (3571,  "WGS 84 / North Pole LAEA Bering Sea",  "Northern hemisphere >45°N",   "metre"),
    (3572,  "WGS 84 / North Pole LAEA Alaska",      "Northern hemisphere >45°N",   "metre"),
    (3573,  "WGS 84 / North Pole LAEA Canada",      "Northern hemisphere >45°N",   "metre"),
    (3574,  "WGS 84 / North Pole LAEA Atlantic",    "Northern hemisphere >45°N",   "metre"),
    (3575,  "WGS 84 / North Pole LAEA Europe",      "Northern hemisphere >45°N",   "metre"),
    (3576,  "WGS 84 / North Pole LAEA Russia",      "Northern hemisphere >45°N",   "metre"),
    (3832,  "WGS 84 / PDC Mercator",                "Pacific region",             "metre"),
    (3833,  "Pulkovo 1942(58) / Gauss-Kruger zone 2", "Central Europe",           "metre"),
    (3834,  "Pulkovo 1942(83) / Gauss-Kruger zone 2", "Central Europe",           "metre"),
    (3835,  "Pulkovo 1942(83) / Gauss-Kruger zone 3", "Central Europe",           "metre"),
    (3836,  "Pulkovo 1942(83) / Gauss-Kruger zone 4", "Central Europe",           "metre"),
    (3837,  "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 3", "Central Europe", "metre"),
    (3838,  "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 4", "Central Europe", "metre"),
    (3839,  "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 9", "Central Europe", "metre"),
    (3840,  "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 10", "Central Europe", "metre"),
    (3841,  "Pulkovo 1942(83) / 3-degree Gauss-Kruger zone 6", "Central Europe", "metre"),
    (3845,  "SWEREF99 / RT90 7.5 gon V emulation", "Sweden",                      "metre"),
    (3846,  "SWEREF99 / RT90 5 gon V emulation",   "Sweden",                      "metre"),
    (3847,  "SWEREF99 / RT90 2.5 gon V emulation", "Sweden",                      "metre"),
    (3848,  "SWEREF99 / RT90 0 gon emulation",     "Sweden",                      "metre"),
    (3849,  "SWEREF99 / RT90 2.5 gon O emulation", "Sweden",                      "metre"),
    (3850,  "SWEREF99 / RT90 5 gon O emulation",   "Sweden",                      "metre"),
    (3986,  "Katanga 1955 / Katanga Gauss zone A",  "DR Congo - Katanga",         "metre"),
    (3987,  "Katanga 1955 / Katanga Gauss zone B",  "DR Congo - Katanga",         "metre"),
    (3988,  "Katanga 1955 / Katanga Gauss zone C",  "DR Congo - Katanga",         "metre"),
    (3989,  "Katanga 1955 / Katanga Gauss zone D",  "DR Congo - Katanga",         "metre"),
    (3991,  "Puerto Rico State Plane CS of 1927",   "Puerto Rico",                "US survey foot"),
    (3992,  "Puerto Rico / St. Croix",              "US Virgin Islands",          "US survey foot"),
    (3994,  "WGS 84 / Mercator 41",                 "Southwestern Pacific",       "metre"),
    (3997,  "WGS 84 / Dubai Local TM",              "UAE - Dubai",                "metre"),
    (54008, "World Sinusoidal",                     "World",                      "metre"),
    (54009, "World Mollweide",                      "World",                      "metre"),
    (54030, "World Robinson",                       "World",                      "metre"),
    // UK / Ireland
    (27700, "OSGB 1936 / British National Grid",    "UK",                          "metre"),
    (29900, "TM65 / Irish National Grid",           "Ireland",                     "metre"),
    (29903, "TM65 / Irish Grid",                    "Ireland",                     "metre"),
    (2157,  "IRENET95 / Irish Transverse Mercator", "Ireland",                    "metre"),
    (2193,  "NZGD2000 / New Zealand Transverse Mercator 2000", "New Zealand",      "metre"),
    (3006,  "SWEREF99 TM",                          "Sweden",                      "metre"),
    (3067,  "ETRS89 / TM35FIN(E,N)",                "Finland",                    "metre"),
    // Germany Gauss-Krüger
    (31466, "DHDN / 3-degree Gauss-Kruger zone 2",  "Germany (6°E band)",          "metre"),
    (31467, "DHDN / 3-degree Gauss-Kruger zone 3",  "Germany (9°E band)",          "metre"),
    (31468, "DHDN / 3-degree Gauss-Kruger zone 4",  "Germany (12°E band)",         "metre"),
    (31469, "DHDN / 3-degree Gauss-Kruger zone 5",  "Germany (15°E band)",         "metre"),
    // Netherlands
    (28992, "Amersfoort / RD New",                  "Netherlands",                 "metre"),
    // France Lambert
    (2154,  "RGF93 v1 / Lambert-93",               "France",                      "metre"),
    (31370, "Belge 1972 / Belgian Lambert 72",      "Belgium",                    "metre"),
    (5514,  "S-JTSK / Krovak East North",           "Czechia, Slovakia",          "metre"),
    // Canada NAD83(CSRS) UTM
    (2955,  "NAD83(CSRS) / UTM zone 11N",           "Canada",                      "metre"),
    (2956,  "NAD83(CSRS) / UTM zone 12N",           "Canada",                      "metre"),
    (2957,  "NAD83(CSRS) / UTM zone 13N",           "Canada",                      "metre"),
    (2958,  "NAD83(CSRS) / UTM zone 17N",           "Canada",                      "metre"),
    (2959,  "NAD83(CSRS) / UTM zone 18N",           "Canada",                      "metre"),
    (2960,  "NAD83(CSRS) / UTM zone 19N",           "Canada",                      "metre"),
    // Australia GDA94 MGA zones 49-56
    (28349, "GDA94 / MGA zone 49",                  "Australia",                   "metre"),
    (28350, "GDA94 / MGA zone 50",                  "Australia",                   "metre"),
    (28351, "GDA94 / MGA zone 51",                  "Australia",                   "metre"),
    (28352, "GDA94 / MGA zone 52",                  "Australia",                   "metre"),
    (28353, "GDA94 / MGA zone 53",                  "Australia",                   "metre"),
    (28354, "GDA94 / MGA zone 54",                  "Australia",                   "metre"),
    (28355, "GDA94 / MGA zone 55",                  "Australia",                   "metre"),
    (28356, "GDA94 / MGA zone 56",                  "Australia",                   "metre"),
    // US State Plane NAD83 (selected — metres)
    (2227,  "NAD83 / California zone 3 (ftUS)",     "California, USA",             "US survey foot"),
    (2229,  "NAD83 / California zone 1",            "California, USA",             "metre"),
    (2230,  "NAD83 / California zone 2",            "California, USA",             "metre"),
    (2231,  "NAD83 / California zone 3",            "California, USA",             "metre"),
    (2232,  "NAD83 / California zone 4",            "California, USA",             "metre"),
    (2233,  "NAD83 / California zone 5",            "California, USA",             "metre"),
    (2234,  "NAD83 / California zone 6",            "California, USA",             "metre"),
    (2236,  "NAD83 / Florida East",                 "Florida East, USA",           "metre"),
    (2237,  "NAD83 / Florida West",                 "Florida West, USA",           "metre"),
    (2238,  "NAD83 / Florida North",                "Florida North, USA",          "metre"),
    (2248,  "NAD83 / Maryland",                     "Maryland, USA",               "metre"),
    (2263,  "NAD83 / New York Long Island",         "New York LI, USA",            "metre"),
    (2272,  "NAD83 / Pennsylvania North",           "Pennsylvania N, USA",         "metre"),
    (2273,  "NAD83 / Pennsylvania South",           "Pennsylvania S, USA",         "metre"),
    (2283,  "NAD83 / Virginia North",               "Virginia N, USA",             "metre"),
    (2284,  "NAD83 / Virginia South",               "Virginia S, USA",             "metre"),
    (2285,  "NAD83 / Washington North",             "Washington N, USA",           "metre"),
    (2286,  "NAD83 / Washington South",             "Washington S, USA",           "metre"),
    // US State Plane NAD83 (national metre codes, EPSG:26929-26998; representable subset)
    (26929, "NAD83 / Alabama East",                 "Alabama East, USA",           "metre"),
    (26930, "NAD83 / Alabama West",                 "Alabama West, USA",           "metre"),
    (26931, "NAD83 / Alaska zone 1",                "Alaska zone 1, USA",          "metre"),
    (26932, "NAD83 / Alaska zone 2",                "Alaska zone 2, USA",          "metre"),
    (26933, "NAD83 / Alaska zone 3",                "Alaska zone 3, USA",          "metre"),
    (26934, "NAD83 / Alaska zone 4",                "Alaska zone 4, USA",          "metre"),
    (26935, "NAD83 / Alaska zone 5",                "Alaska zone 5, USA",          "metre"),
    (26936, "NAD83 / Alaska zone 6",                "Alaska zone 6, USA",          "metre"),
    (26937, "NAD83 / Alaska zone 7",                "Alaska zone 7, USA",          "metre"),
    (26938, "NAD83 / Alaska zone 8",                "Alaska zone 8, USA",          "metre"),
    (26939, "NAD83 / Alaska zone 9",                "Alaska zone 9, USA",          "metre"),
    (26940, "NAD83 / Alaska zone 10",               "Alaska zone 10, USA",         "metre"),
    (26941, "NAD83 / California zone 1",            "California, USA",             "metre"),
    (26942, "NAD83 / California zone 2",            "California, USA",             "metre"),
    (26943, "NAD83 / California zone 3",            "California, USA",             "metre"),
    (26944, "NAD83 / California zone 4",            "California, USA",             "metre"),
    (26945, "NAD83 / California zone 5",            "California, USA",             "metre"),
    (26946, "NAD83 / California zone 6",            "California, USA",             "metre"),
    (26948, "NAD83 / Arizona East",                 "Arizona East, USA",           "metre"),
    (26949, "NAD83 / Arizona Central",              "Arizona Central, USA",        "metre"),
    (26950, "NAD83 / Arizona West",                 "Arizona West, USA",           "metre"),
    (26951, "NAD83 / Arkansas North",               "Arkansas North, USA",         "metre"),
    (26952, "NAD83 / Arkansas South",               "Arkansas South, USA",         "metre"),
    (26953, "NAD83 / Colorado North",               "Colorado North, USA",         "metre"),
    (26954, "NAD83 / Colorado Central",             "Colorado Central, USA",       "metre"),
    (26955, "NAD83 / Colorado South",               "Colorado South, USA",         "metre"),
    (26956, "NAD83 / Connecticut",                  "Connecticut, USA",            "metre"),
    (26957, "NAD83 / Delaware",                     "Delaware, USA",               "metre"),
    (26958, "NAD83 / Florida East",                 "Florida East, USA",           "metre"),
    (26959, "NAD83 / Florida West",                 "Florida West, USA",           "metre"),
    (26960, "NAD83 / Florida North",                "Florida North, USA",          "metre"),
    (26961, "NAD83 / Hawaii zone 1",                "Hawaii zone 1, USA",          "metre"),
    (26962, "NAD83 / Hawaii zone 2",                "Hawaii zone 2, USA",          "metre"),
    (26963, "NAD83 / Hawaii zone 3",                "Hawaii zone 3, USA",          "metre"),
    (26964, "NAD83 / Hawaii zone 4",                "Hawaii zone 4, USA",          "metre"),
    (26965, "NAD83 / Hawaii zone 5",                "Hawaii zone 5, USA",          "metre"),
    (26966, "NAD83 / Georgia East",                 "Georgia East, USA",           "metre"),
    (26967, "NAD83 / Georgia West",                 "Georgia West, USA",           "metre"),
    (26968, "NAD83 / Idaho East",                   "Idaho East, USA",             "metre"),
    (26969, "NAD83 / Idaho Central",                "Idaho Central, USA",          "metre"),
    (26970, "NAD83 / Idaho West",                   "Idaho West, USA",             "metre"),
    (26971, "NAD83 / Illinois East",                "Illinois East, USA",          "metre"),
    (26972, "NAD83 / Illinois West",                "Illinois West, USA",          "metre"),
    (26973, "NAD83 / Indiana East",                 "Indiana East, USA",           "metre"),
    (26974, "NAD83 / Indiana West",                 "Indiana West, USA",           "metre"),
    (26975, "NAD83 / Iowa North",                   "Iowa North, USA",             "metre"),
    (26976, "NAD83 / Iowa South",                   "Iowa South, USA",             "metre"),
    (26977, "NAD83 / Kansas North",                 "Kansas North, USA",           "metre"),
    (26978, "NAD83 / Kansas South",                 "Kansas South, USA",           "metre"),
    (26979, "NAD83 / Kentucky North",               "Kentucky North, USA",         "metre"),
    (26980, "NAD83 / Kentucky South",               "Kentucky South, USA",         "metre"),
    (26981, "NAD83 / Louisiana North",              "Louisiana North, USA",        "metre"),
    (26982, "NAD83 / Louisiana South",              "Louisiana South, USA",        "metre"),
    (26983, "NAD83 / Maine East",                   "Maine East, USA",             "metre"),
    (26984, "NAD83 / Maine West",                   "Maine West, USA",             "metre"),
    (26985, "NAD83 / Maryland",                     "Maryland, USA",               "metre"),
    (26986, "NAD83 / Massachusetts Mainland",       "Massachusetts Mainland, USA", "metre"),
    (26987, "NAD83 / Massachusetts Island",         "Massachusetts Island, USA",   "metre"),
    (26988, "NAD83 / Michigan North",               "Michigan North, USA",         "metre"),
    (26989, "NAD83 / Michigan Central",             "Michigan Central, USA",       "metre"),
    (26990, "NAD83 / Michigan South",               "Michigan South, USA",         "metre"),
    (26991, "NAD83 / Minnesota North",              "Minnesota North, USA",        "metre"),
    (26992, "NAD83 / Minnesota Central",            "Minnesota Central, USA",      "metre"),
    (26993, "NAD83 / Minnesota South",              "Minnesota South, USA",        "metre"),
    (26994, "NAD83 / Mississippi East",             "Mississippi East, USA",       "metre"),
    (26995, "NAD83 / Mississippi West",             "Mississippi West, USA",       "metre"),
    (26996, "NAD83 / Missouri East",                "Missouri East, USA",          "metre"),
    (26997, "NAD83 / Missouri Central",             "Missouri Central, USA",       "metre"),
    (26998, "NAD83 / Missouri West",                "Missouri West, USA",          "metre"),
    // US State Plane NAD83(NSRS2007) (EPSG:3465-3552)
    (3465, "NAD83(NSRS2007) / Alabama East", "USA", "metre"),
    (3466, "NAD83(NSRS2007) / Alabama West", "USA", "metre"),
    (3467, "NAD83(NSRS2007) / Alaska Albers", "USA", "metre"),
    (3468, "NAD83(NSRS2007) / Alaska zone 1", "USA", "metre"),
    (3469, "NAD83(NSRS2007) / Alaska zone 2", "USA", "metre"),
    (3470, "NAD83(NSRS2007) / Alaska zone 3", "USA", "metre"),
    (3471, "NAD83(NSRS2007) / Alaska zone 4", "USA", "metre"),
    (3472, "NAD83(NSRS2007) / Alaska zone 5", "USA", "metre"),
    (3473, "NAD83(NSRS2007) / Alaska zone 6", "USA", "metre"),
    (3474, "NAD83(NSRS2007) / Alaska zone 7", "USA", "metre"),
    (3475, "NAD83(NSRS2007) / Alaska zone 8", "USA", "metre"),
    (3476, "NAD83(NSRS2007) / Alaska zone 9", "USA", "metre"),
    (3477, "NAD83(NSRS2007) / Alaska zone 10", "USA", "metre"),
    (3478, "NAD83(NSRS2007) / Arizona Central", "USA", "metre"),
    (3479, "NAD83(NSRS2007) / Arizona Central (ft)", "USA", "foot"),
    (3480, "NAD83(NSRS2007) / Arizona East", "USA", "metre"),
    (3481, "NAD83(NSRS2007) / Arizona East (ft)", "USA", "foot"),
    (3482, "NAD83(NSRS2007) / Arizona West", "USA", "metre"),
    (3483, "NAD83(NSRS2007) / Arizona West (ft)", "USA", "foot"),
    (3484, "NAD83(NSRS2007) / Arkansas North", "USA", "metre"),
    (3485, "NAD83(NSRS2007) / Arkansas North (ftUS)", "USA", "US survey foot"),
    (3486, "NAD83(NSRS2007) / Arkansas South", "USA", "metre"),
    (3487, "NAD83(NSRS2007) / Arkansas South (ftUS)", "USA", "US survey foot"),
    (3488, "NAD83(NSRS2007) / California Albers", "USA", "metre"),
    (3489, "NAD83(NSRS2007) / California zone 1", "USA", "metre"),
    (3490, "NAD83(NSRS2007) / California zone 1 (ftUS)", "USA", "US survey foot"),
    (3491, "NAD83(NSRS2007) / California zone 2", "USA", "metre"),
    (3492, "NAD83(NSRS2007) / California zone 2 (ftUS)", "USA", "US survey foot"),
    (3493, "NAD83(NSRS2007) / California zone 3", "USA", "metre"),
    (3494, "NAD83(NSRS2007) / California zone 3 (ftUS)", "USA", "US survey foot"),
    (3495, "NAD83(NSRS2007) / California zone 4", "USA", "metre"),
    (3496, "NAD83(NSRS2007) / California zone 4 (ftUS)", "USA", "US survey foot"),
    (3497, "NAD83(NSRS2007) / California zone 5", "USA", "metre"),
    (3498, "NAD83(NSRS2007) / California zone 5 (ftUS)", "USA", "US survey foot"),
    (3499, "NAD83(NSRS2007) / California zone 6", "USA", "metre"),
    (3500, "NAD83(NSRS2007) / California zone 6 (ftUS)", "USA", "US survey foot"),
    (3501, "NAD83(NSRS2007) / Colorado Central", "USA", "metre"),
    (3503, "NAD83(NSRS2007) / Colorado North", "USA", "metre"),
    (3504, "NAD83(NSRS2007) / Colorado North (ftUS)", "USA", "US survey foot"),
    (3505, "NAD83(NSRS2007) / Colorado South", "USA", "metre"),
    (3506, "NAD83(NSRS2007) / Colorado South (ftUS)", "USA", "US survey foot"),
    (3507, "NAD83(NSRS2007) / Connecticut", "USA", "metre"),
    (3508, "NAD83(NSRS2007) / Connecticut (ftUS)", "USA", "US survey foot"),
    (3509, "NAD83(NSRS2007) / Delaware", "USA", "metre"),
    (3510, "NAD83(NSRS2007) / Delaware (ftUS)", "USA", "US survey foot"),
    (3511, "NAD83(NSRS2007) / Florida East", "USA", "metre"),
    (3512, "NAD83(NSRS2007) / Florida East (ftUS)", "USA", "US survey foot"),
    (3513, "NAD83(NSRS2007) / Florida GDL Albers", "USA", "metre"),
    (3514, "NAD83(NSRS2007) / Florida North", "USA", "metre"),
    (3515, "NAD83(NSRS2007) / Florida North (ftUS)", "USA", "US survey foot"),
    (3516, "NAD83(NSRS2007) / Florida West", "USA", "metre"),
    (3517, "NAD83(NSRS2007) / Florida West (ftUS)", "USA", "US survey foot"),
    (3518, "NAD83(NSRS2007) / Georgia East", "USA", "metre"),
    (3519, "NAD83(NSRS2007) / Georgia East (ftUS)", "USA", "US survey foot"),
    (3520, "NAD83(NSRS2007) / Georgia West", "USA", "metre"),
    (3521, "NAD83(NSRS2007) / Georgia West (ftUS)", "USA", "US survey foot"),
    (3522, "NAD83(NSRS2007) / Idaho Central", "USA", "metre"),
    (3523, "NAD83(NSRS2007) / Idaho Central (ftUS)", "USA", "US survey foot"),
    (3524, "NAD83(NSRS2007) / Idaho East", "USA", "metre"),
    (3525, "NAD83(NSRS2007) / Idaho East (ftUS)", "USA", "US survey foot"),
    (3526, "NAD83(NSRS2007) / Idaho West", "USA", "metre"),
    (3527, "NAD83(NSRS2007) / Idaho West (ftUS)", "USA", "US survey foot"),
    (3528, "NAD83(NSRS2007) / Illinois East", "USA", "metre"),
    (3529, "NAD83(NSRS2007) / Illinois East (ftUS)", "USA", "US survey foot"),
    (3530, "NAD83(NSRS2007) / Illinois West", "USA", "metre"),
    (3531, "NAD83(NSRS2007) / Illinois West (ftUS)", "USA", "US survey foot"),
    (3532, "NAD83(NSRS2007) / Indiana East", "USA", "metre"),
    (3533, "NAD83(NSRS2007) / Indiana East (ftUS)", "USA", "US survey foot"),
    (3534, "NAD83(NSRS2007) / Indiana West", "USA", "metre"),
    (3535, "NAD83(NSRS2007) / Indiana West (ftUS)", "USA", "US survey foot"),
    (3536, "NAD83(NSRS2007) / Iowa North", "USA", "metre"),
    (3537, "NAD83(NSRS2007) / Iowa North (ftUS)", "USA", "US survey foot"),
    (3538, "NAD83(NSRS2007) / Iowa South", "USA", "metre"),
    (3539, "NAD83(NSRS2007) / Iowa South (ftUS)", "USA", "US survey foot"),
    (3540, "NAD83(NSRS2007) / Kansas North", "USA", "metre"),
    (3541, "NAD83(NSRS2007) / Kansas North (ftUS)", "USA", "US survey foot"),
    (3542, "NAD83(NSRS2007) / Kansas South", "USA", "metre"),
    (3543, "NAD83(NSRS2007) / Kansas South (ftUS)", "USA", "US survey foot"),
    (3544, "NAD83(NSRS2007) / Kentucky North", "USA", "metre"),
    (3545, "NAD83(NSRS2007) / Kentucky North (ftUS)", "USA", "US survey foot"),
    (3546, "NAD83(NSRS2007) / Kentucky Single Zone", "USA", "metre"),
    (3547, "NAD83(NSRS2007) / Kentucky Single Zone (ftUS)", "USA", "US survey foot"),
    (3548, "NAD83(NSRS2007) / Kentucky South", "USA", "metre"),
    (3549, "NAD83(NSRS2007) / Kentucky South (ftUS)", "USA", "US survey foot"),
    (3550, "NAD83(NSRS2007) / Louisiana North", "USA", "metre"),
    (3551, "NAD83(NSRS2007) / Louisiana North (ftUS)", "USA", "US survey foot"),
    (3552, "NAD83(NSRS2007) / Louisiana South", "USA", "metre"),
    // NAD83(2011) UTM (active EPSG set)
    (6328, "NAD83(2011) / UTM zone 59N", "USA and territories", "metre"),
    (6329, "NAD83(2011) / UTM zone 60N", "USA and territories", "metre"),
    (6330, "NAD83(2011) / UTM zone 1N",  "USA and territories", "metre"),
    (6331, "NAD83(2011) / UTM zone 2N",  "USA and territories", "metre"),
    (6332, "NAD83(2011) / UTM zone 3N",  "USA and territories", "metre"),
    (6333, "NAD83(2011) / UTM zone 4N",  "USA and territories", "metre"),
    (6334, "NAD83(2011) / UTM zone 5N",  "USA and territories", "metre"),
    (6335, "NAD83(2011) / UTM zone 6N",  "USA and territories", "metre"),
    (6336, "NAD83(2011) / UTM zone 7N",  "USA and territories", "metre"),
    (6337, "NAD83(2011) / UTM zone 8N",  "USA and territories", "metre"),
    (6338, "NAD83(2011) / UTM zone 9N",  "USA and territories", "metre"),
    (6339, "NAD83(2011) / UTM zone 10N", "USA and territories", "metre"),
    (6340, "NAD83(2011) / UTM zone 11N", "USA and territories", "metre"),
    (6341, "NAD83(2011) / UTM zone 12N", "USA and territories", "metre"),
    (6342, "NAD83(2011) / UTM zone 13N", "USA and territories", "metre"),
    (6343, "NAD83(2011) / UTM zone 14N", "USA and territories", "metre"),
    (6344, "NAD83(2011) / UTM zone 15N", "USA and territories", "metre"),
    (6345, "NAD83(2011) / UTM zone 16N", "USA and territories", "metre"),
    (6346, "NAD83(2011) / UTM zone 17N", "USA and territories", "metre"),
    (6347, "NAD83(2011) / UTM zone 18N", "USA and territories", "metre"),
    (6348, "NAD83(2011) / UTM zone 19N", "USA and territories", "metre"),
    // US State Plane NAD83(2011) (EPSG:6355-6627, implementable subset)
    (6355, "NAD83(2011) / Alabama East", "USA", "metre"),
    (6356, "NAD83(2011) / Alabama West", "USA", "metre"),
    (6393, "NAD83(2011) / Alaska Albers", "USA", "metre"),
    (6394, "NAD83(2011) / Alaska zone 1", "USA", "metre"),
    (6395, "NAD83(2011) / Alaska zone 2", "USA", "metre"),
    (6396, "NAD83(2011) / Alaska zone 3", "USA", "metre"),
    (6397, "NAD83(2011) / Alaska zone 4", "USA", "metre"),
    (6398, "NAD83(2011) / Alaska zone 5", "USA", "metre"),
    (6399, "NAD83(2011) / Alaska zone 6", "USA", "metre"),
    (6400, "NAD83(2011) / Alaska zone 7", "USA", "metre"),
    (6401, "NAD83(2011) / Alaska zone 8", "USA", "metre"),
    (6402, "NAD83(2011) / Alaska zone 9", "USA", "metre"),
    (6403, "NAD83(2011) / Alaska zone 10", "USA", "metre"),
    (6404, "NAD83(2011) / Arizona Central", "USA", "metre"),
    (6405, "NAD83(2011) / Arizona Central (ft)", "USA", "foot"),
    (6406, "NAD83(2011) / Arizona East", "USA", "metre"),
    (6407, "NAD83(2011) / Arizona East (ft)", "USA", "foot"),
    (6408, "NAD83(2011) / Arizona West", "USA", "metre"),
    (6409, "NAD83(2011) / Arizona West (ft)", "USA", "foot"),
    (6410, "NAD83(2011) / Arkansas North", "USA", "metre"),
    (6411, "NAD83(2011) / Arkansas North (ftUS)", "USA", "US survey foot"),
    (6412, "NAD83(2011) / Arkansas South", "USA", "metre"),
    (6413, "NAD83(2011) / Arkansas South (ftUS)", "USA", "US survey foot"),
    (6414, "NAD83(2011) / California Albers", "USA", "metre"),
    (6415, "NAD83(2011) / California zone 1", "USA", "metre"),
    (6416, "NAD83(2011) / California zone 1 (ftUS)", "USA", "US survey foot"),
    (6417, "NAD83(2011) / California zone 2", "USA", "metre"),
    (6418, "NAD83(2011) / California zone 2 (ftUS)", "USA", "US survey foot"),
    (6419, "NAD83(2011) / California zone 3", "USA", "metre"),
    (6420, "NAD83(2011) / California zone 3 (ftUS)", "USA", "US survey foot"),
    (6421, "NAD83(2011) / California zone 4", "USA", "metre"),
    (6422, "NAD83(2011) / California zone 4 (ftUS)", "USA", "US survey foot"),
    (6423, "NAD83(2011) / California zone 5", "USA", "metre"),
    (6424, "NAD83(2011) / California zone 5 (ftUS)", "USA", "US survey foot"),
    (6425, "NAD83(2011) / California zone 6", "USA", "metre"),
    (6426, "NAD83(2011) / California zone 6 (ftUS)", "USA", "US survey foot"),
    (6427, "NAD83(2011) / Colorado Central", "USA", "metre"),
    (6428, "NAD83(2011) / Colorado Central (ftUS)", "USA", "US survey foot"),
    (6429, "NAD83(2011) / Colorado North", "USA", "metre"),
    (6430, "NAD83(2011) / Colorado North (ftUS)", "USA", "US survey foot"),
    (6431, "NAD83(2011) / Colorado South", "USA", "metre"),
    (6432, "NAD83(2011) / Colorado South (ftUS)", "USA", "US survey foot"),
    (6433, "NAD83(2011) / Connecticut", "USA", "metre"),
    (6434, "NAD83(2011) / Connecticut (ftUS)", "USA", "US survey foot"),
    (6435, "NAD83(2011) / Delaware", "USA", "metre"),
    (6436, "NAD83(2011) / Delaware (ftUS)", "USA", "US survey foot"),
    (6437, "NAD83(2011) / Florida East", "USA", "metre"),
    (6438, "NAD83(2011) / Florida East (ftUS)", "USA", "US survey foot"),
    (6439, "NAD83(2011) / Florida GDL Albers", "USA", "metre"),
    (6440, "NAD83(2011) / Florida North", "USA", "metre"),
    (6441, "NAD83(2011) / Florida North (ftUS)", "USA", "US survey foot"),
    (6442, "NAD83(2011) / Florida West", "USA", "metre"),
    (6443, "NAD83(2011) / Florida West (ftUS)", "USA", "US survey foot"),
    (6444, "NAD83(2011) / Georgia East", "USA", "metre"),
    (6445, "NAD83(2011) / Georgia East (ftUS)", "USA", "US survey foot"),
    (6446, "NAD83(2011) / Georgia West", "USA", "metre"),
    (6447, "NAD83(2011) / Georgia West (ftUS)", "USA", "US survey foot"),
    (6448, "NAD83(2011) / Idaho Central", "USA", "metre"),
    (6449, "NAD83(2011) / Idaho Central (ftUS)", "USA", "US survey foot"),
    (6450, "NAD83(2011) / Idaho East", "USA", "metre"),
    (6451, "NAD83(2011) / Idaho East (ftUS)", "USA", "US survey foot"),
    (6452, "NAD83(2011) / Idaho West", "USA", "metre"),
    (6453, "NAD83(2011) / Idaho West (ftUS)", "USA", "US survey foot"),
    (6454, "NAD83(2011) / Illinois East", "USA", "metre"),
    (6455, "NAD83(2011) / Illinois East (ftUS)", "USA", "US survey foot"),
    (6456, "NAD83(2011) / Illinois West", "USA", "metre"),
    (6457, "NAD83(2011) / Illinois West (ftUS)", "USA", "US survey foot"),
    (6458, "NAD83(2011) / Indiana East", "USA", "metre"),
    (6459, "NAD83(2011) / Indiana East (ftUS)", "USA", "US survey foot"),
    (6460, "NAD83(2011) / Indiana West", "USA", "metre"),
    (6461, "NAD83(2011) / Indiana West (ftUS)", "USA", "US survey foot"),
    (6462, "NAD83(2011) / Iowa North", "USA", "metre"),
    (6463, "NAD83(2011) / Iowa North (ftUS)", "USA", "US survey foot"),
    (6464, "NAD83(2011) / Iowa South", "USA", "metre"),
    (6465, "NAD83(2011) / Iowa South (ftUS)", "USA", "US survey foot"),
    (6466, "NAD83(2011) / Kansas North", "USA", "metre"),
    (6467, "NAD83(2011) / Kansas North (ftUS)", "USA", "US survey foot"),
    (6468, "NAD83(2011) / Kansas South", "USA", "metre"),
    (6469, "NAD83(2011) / Kansas South (ftUS)", "USA", "US survey foot"),
    (6470, "NAD83(2011) / Kentucky North", "USA", "metre"),
    (6471, "NAD83(2011) / Kentucky North (ftUS)", "USA", "US survey foot"),
    (6472, "NAD83(2011) / Kentucky Single Zone", "USA", "metre"),
    (6473, "NAD83(2011) / Kentucky Single Zone (ftUS)", "USA", "US survey foot"),
    (6474, "NAD83(2011) / Kentucky South", "USA", "metre"),
    (6475, "NAD83(2011) / Kentucky South (ftUS)", "USA", "US survey foot"),
    (6476, "NAD83(2011) / Louisiana North", "USA", "metre"),
    (6477, "NAD83(2011) / Louisiana North (ftUS)", "USA", "US survey foot"),
    (6478, "NAD83(2011) / Louisiana South", "USA", "metre"),
    (6479, "NAD83(2011) / Louisiana South (ftUS)", "USA", "US survey foot"),
    (6480, "NAD83(2011) / Maine CS2000 Central", "USA", "metre"),
    (6481, "NAD83(2011) / Maine CS2000 East", "USA", "metre"),
    (6482, "NAD83(2011) / Maine CS2000 West", "USA", "metre"),
    (6483, "NAD83(2011) / Maine East", "USA", "metre"),
    (6484, "NAD83(2011) / Maine East (ftUS)", "USA", "US survey foot"),
    (6485, "NAD83(2011) / Maine West", "USA", "metre"),
    (6486, "NAD83(2011) / Maine West (ftUS)", "USA", "US survey foot"),
    (6487, "NAD83(2011) / Maryland", "USA", "metre"),
    (6488, "NAD83(2011) / Maryland (ftUS)", "USA", "US survey foot"),
    (6489, "NAD83(2011) / Massachusetts Island", "USA", "metre"),
    (6490, "NAD83(2011) / Massachusetts Island (ftUS)", "USA", "US survey foot"),
    (6491, "NAD83(2011) / Massachusetts Mainland", "USA", "metre"),
    (6492, "NAD83(2011) / Massachusetts Mainland (ftUS)", "USA", "US survey foot"),
    (6493, "NAD83(2011) / Michigan Central", "USA", "metre"),
    (6494, "NAD83(2011) / Michigan Central (ft)", "USA", "foot"),
    (6495, "NAD83(2011) / Michigan North", "USA", "metre"),
    (6496, "NAD83(2011) / Michigan North (ft)", "USA", "foot"),
    (6497, "NAD83(2011) / Michigan Oblique Mercator", "USA", "metre"),
    (6498, "NAD83(2011) / Michigan South", "USA", "metre"),
    (6499, "NAD83(2011) / Michigan South (ft)", "USA", "foot"),
    (6500, "NAD83(2011) / Minnesota Central", "USA", "metre"),
    (6501, "NAD83(2011) / Minnesota Central (ftUS)", "USA", "US survey foot"),
    (6502, "NAD83(2011) / Minnesota North", "USA", "metre"),
    (6503, "NAD83(2011) / Minnesota North (ftUS)", "USA", "US survey foot"),
    (6504, "NAD83(2011) / Minnesota South", "USA", "metre"),
    (6505, "NAD83(2011) / Minnesota South (ftUS)", "USA", "US survey foot"),
    (6506, "NAD83(2011) / Mississippi East", "USA", "metre"),
    (6507, "NAD83(2011) / Mississippi East (ftUS)", "USA", "US survey foot"),
    (6508, "NAD83(2011) / Mississippi TM", "USA", "metre"),
    (6509, "NAD83(2011) / Mississippi West", "USA", "metre"),
    (6510, "NAD83(2011) / Mississippi West (ftUS)", "USA", "US survey foot"),
    (6511, "NAD83(2011) / Missouri Central", "USA", "metre"),
    (6512, "NAD83(2011) / Missouri East", "USA", "metre"),
    (6513, "NAD83(2011) / Missouri West", "USA", "metre"),
    (6514, "NAD83(2011) / Montana", "USA", "metre"),
    (6515, "NAD83(2011) / Montana (ft)", "USA", "foot"),
    (6516, "NAD83(2011) / Nebraska", "USA", "metre"),
    (6517, "NAD83(2011) / Nebraska (ftUS)", "USA", "US survey foot"),
    (6518, "NAD83(2011) / Nevada Central", "USA", "metre"),
    (6519, "NAD83(2011) / Nevada Central (ftUS)", "USA", "US survey foot"),
    (6520, "NAD83(2011) / Nevada East", "USA", "metre"),
    (6521, "NAD83(2011) / Nevada East (ftUS)", "USA", "US survey foot"),
    (6522, "NAD83(2011) / Nevada West", "USA", "metre"),
    (6523, "NAD83(2011) / Nevada West (ftUS)", "USA", "US survey foot"),
    (6524, "NAD83(2011) / New Hampshire", "USA", "metre"),
    (6525, "NAD83(2011) / New Hampshire (ftUS)", "USA", "US survey foot"),
    (6526, "NAD83(2011) / New Jersey", "USA", "metre"),
    (6527, "NAD83(2011) / New Jersey (ftUS)", "USA", "US survey foot"),
    (6528, "NAD83(2011) / New Mexico Central", "USA", "metre"),
    (6529, "NAD83(2011) / New Mexico Central (ftUS)", "USA", "US survey foot"),
    (6530, "NAD83(2011) / New Mexico East", "USA", "metre"),
    (6531, "NAD83(2011) / New Mexico East (ftUS)", "USA", "US survey foot"),
    (6532, "NAD83(2011) / New Mexico West", "USA", "metre"),
    (6533, "NAD83(2011) / New Mexico West (ftUS)", "USA", "US survey foot"),
    (6534, "NAD83(2011) / New York Central", "USA", "metre"),
    (6535, "NAD83(2011) / New York Central (ftUS)", "USA", "US survey foot"),
    (6536, "NAD83(2011) / New York East", "USA", "metre"),
    (6537, "NAD83(2011) / New York East (ftUS)", "USA", "US survey foot"),
    (6538, "NAD83(2011) / New York Long Island", "USA", "metre"),
    (6539, "NAD83(2011) / New York Long Island (ftUS)", "USA", "US survey foot"),
    (6540, "NAD83(2011) / New York West", "USA", "metre"),
    (6541, "NAD83(2011) / New York West (ftUS)", "USA", "US survey foot"),
    (6542, "NAD83(2011) / North Carolina", "USA", "metre"),
    (6543, "NAD83(2011) / North Carolina (ftUS)", "USA", "US survey foot"),
    (6544, "NAD83(2011) / North Dakota North", "USA", "metre"),
    (6545, "NAD83(2011) / North Dakota North (ft)", "USA", "foot"),
    (6546, "NAD83(2011) / North Dakota South", "USA", "metre"),
    (6547, "NAD83(2011) / North Dakota South (ft)", "USA", "foot"),
    (6548, "NAD83(2011) / Ohio North", "USA", "metre"),
    (6549, "NAD83(2011) / Ohio North (ftUS)", "USA", "US survey foot"),
    (6550, "NAD83(2011) / Ohio South", "USA", "metre"),
    (6551, "NAD83(2011) / Ohio South (ftUS)", "USA", "US survey foot"),
    (6552, "NAD83(2011) / Oklahoma North", "USA", "metre"),
    (6553, "NAD83(2011) / Oklahoma North (ftUS)", "USA", "US survey foot"),
    (6554, "NAD83(2011) / Oklahoma South", "USA", "metre"),
    (6555, "NAD83(2011) / Oklahoma South (ftUS)", "USA", "US survey foot"),
    (6556, "NAD83(2011) / Oregon LCC (m)", "USA", "metre"),
    (6557, "NAD83(2011) / Oregon GIC Lambert (ft)", "USA", "foot"),
    (6558, "NAD83(2011) / Oregon North", "USA", "metre"),
    (6559, "NAD83(2011) / Oregon North (ft)", "USA", "foot"),
    (6560, "NAD83(2011) / Oregon South", "USA", "metre"),
    (6561, "NAD83(2011) / Oregon South (ft)", "USA", "foot"),
    (6562, "NAD83(2011) / Pennsylvania North", "USA", "metre"),
    (6563, "NAD83(2011) / Pennsylvania North (ftUS)", "USA", "US survey foot"),
    (6564, "NAD83(2011) / Pennsylvania South", "USA", "metre"),
    (6565, "NAD83(2011) / Pennsylvania South (ftUS)", "USA", "US survey foot"),
    (6566, "NAD83(2011) / Puerto Rico and Virgin Is.", "USA", "metre"),
    (6567, "NAD83(2011) / Rhode Island", "USA", "metre"),
    (6568, "NAD83(2011) / Rhode Island (ftUS)", "USA", "US survey foot"),
    (6569, "NAD83(2011) / South Carolina", "USA", "metre"),
    (6570, "NAD83(2011) / South Carolina (ft)", "USA", "foot"),
    (6571, "NAD83(2011) / South Dakota North", "USA", "metre"),
    (6572, "NAD83(2011) / South Dakota North (ftUS)", "USA", "US survey foot"),
    (6573, "NAD83(2011) / South Dakota South", "USA", "metre"),
    (6574, "NAD83(2011) / South Dakota South (ftUS)", "USA", "US survey foot"),
    (6575, "NAD83(2011) / Tennessee", "USA", "metre"),
    (6576, "NAD83(2011) / Tennessee (ftUS)", "USA", "US survey foot"),
    (6577, "NAD83(2011) / Texas Central", "USA", "metre"),
    (6578, "NAD83(2011) / Texas Central (ftUS)", "USA", "US survey foot"),
    (6579, "NAD83(2011) / Texas Centric Albers Equal Area", "USA", "metre"),
    (6580, "NAD83(2011) / Texas Centric Lambert Conformal", "USA", "metre"),
    (6581, "NAD83(2011) / Texas North", "USA", "metre"),
    (6582, "NAD83(2011) / Texas North (ftUS)", "USA", "US survey foot"),
    (6583, "NAD83(2011) / Texas North Central", "USA", "metre"),
    (6584, "NAD83(2011) / Texas North Central (ftUS)", "USA", "US survey foot"),
    (6585, "NAD83(2011) / Texas South", "USA", "metre"),
    (6586, "NAD83(2011) / Texas South (ftUS)", "USA", "US survey foot"),
    (6587, "NAD83(2011) / Texas South Central", "USA", "metre"),
    (6588, "NAD83(2011) / Texas South Central (ftUS)", "USA", "US survey foot"),
    (6589, "NAD83(2011) / Vermont", "USA", "metre"),
    (6590, "NAD83(2011) / Vermont (ftUS)", "USA", "US survey foot"),
    (6591, "NAD83(2011) / Virginia Lambert", "USA", "metre"),
    (6592, "NAD83(2011) / Virginia North", "USA", "metre"),
    (6593, "NAD83(2011) / Virginia North (ftUS)", "USA", "US survey foot"),
    (6594, "NAD83(2011) / Virginia South", "USA", "metre"),
    (6595, "NAD83(2011) / Virginia South (ftUS)", "USA", "US survey foot"),
    (6596, "NAD83(2011) / Washington North", "USA", "metre"),
    (6597, "NAD83(2011) / Washington North (ftUS)", "USA", "US survey foot"),
    (6598, "NAD83(2011) / Washington South", "USA", "metre"),
    (6599, "NAD83(2011) / Washington South (ftUS)", "USA", "US survey foot"),
    (6600, "NAD83(2011) / West Virginia North", "USA", "metre"),
    (6601, "NAD83(2011) / West Virginia North (ftUS)", "USA", "US survey foot"),
    (6602, "NAD83(2011) / West Virginia South", "USA", "metre"),
    (6603, "NAD83(2011) / West Virginia South (ftUS)", "USA", "US survey foot"),
    (6604, "NAD83(2011) / Wisconsin Central", "USA", "metre"),
    (6605, "NAD83(2011) / Wisconsin Central (ftUS)", "USA", "US survey foot"),
    (6606, "NAD83(2011) / Wisconsin North", "USA", "metre"),
    (6607, "NAD83(2011) / Wisconsin North (ftUS)", "USA", "US survey foot"),
    (6608, "NAD83(2011) / Wisconsin South", "USA", "metre"),
    (6609, "NAD83(2011) / Wisconsin South (ftUS)", "USA", "US survey foot"),
    (6610, "NAD83(2011) / Wisconsin Transverse Mercator", "USA", "metre"),
    (6611, "NAD83(2011) / Wyoming East", "USA", "metre"),
    (6612, "NAD83(2011) / Wyoming East (ftUS)", "USA", "US survey foot"),
    (6613, "NAD83(2011) / Wyoming East Central", "USA", "metre"),
    (6614, "NAD83(2011) / Wyoming East Central (ftUS)", "USA", "US survey foot"),
    (6615, "NAD83(2011) / Wyoming West", "USA", "metre"),
    (6616, "NAD83(2011) / Wyoming West (ftUS)", "USA", "US survey foot"),
    (6617, "NAD83(2011) / Wyoming West Central", "USA", "metre"),
    (6618, "NAD83(2011) / Wyoming West Central (ftUS)", "USA", "US survey foot"),
    (6619, "NAD83(2011) / Utah Central", "USA", "metre"),
    (6620, "NAD83(2011) / Utah North", "USA", "metre"),
    (6621, "NAD83(2011) / Utah South", "USA", "metre"),
    (6625, "NAD83(2011) / Utah Central (ftUS)", "USA", "US survey foot"),
    (6626, "NAD83(2011) / Utah North (ftUS)", "USA", "US survey foot"),
    (6627, "NAD83(2011) / Utah South (ftUS)", "USA", "US survey foot"),
    // US State Plane NAD83(HARN) (national metre codes, EPSG:2759-2866)
    (2759,  "NAD83(HARN) / Alabama East",            "Alabama East, USA",           "metre"),
    (2760,  "NAD83(HARN) / Alabama West",            "Alabama West, USA",           "metre"),
    (2761,  "NAD83(HARN) / Arizona East",            "Arizona East, USA",           "metre"),
    (2762,  "NAD83(HARN) / Arizona Central",         "Arizona Central, USA",        "metre"),
    (2763,  "NAD83(HARN) / Arizona West",            "Arizona West, USA",           "metre"),
    (2764,  "NAD83(HARN) / Arkansas North",          "Arkansas North, USA",         "metre"),
    (2765,  "NAD83(HARN) / Arkansas South",          "Arkansas South, USA",         "metre"),
    (2766,  "NAD83(HARN) / California zone 1",       "California, USA",             "metre"),
    (2767,  "NAD83(HARN) / California zone 2",       "California, USA",             "metre"),
    (2768,  "NAD83(HARN) / California zone 3",       "California, USA",             "metre"),
    (2769,  "NAD83(HARN) / California zone 4",       "California, USA",             "metre"),
    (2770,  "NAD83(HARN) / California zone 5",       "California, USA",             "metre"),
    (2771,  "NAD83(HARN) / California zone 6",       "California, USA",             "metre"),
    (2772,  "NAD83(HARN) / Colorado North",          "Colorado North, USA",         "metre"),
    (2773,  "NAD83(HARN) / Colorado Central",        "Colorado Central, USA",       "metre"),
    (2774,  "NAD83(HARN) / Colorado South",          "Colorado South, USA",         "metre"),
    (2775,  "NAD83(HARN) / Connecticut",             "Connecticut, USA",            "metre"),
    (2776,  "NAD83(HARN) / Delaware",                "Delaware, USA",               "metre"),
    (2777,  "NAD83(HARN) / Florida East",            "Florida East, USA",           "metre"),
    (2778,  "NAD83(HARN) / Florida West",            "Florida West, USA",           "metre"),
    (2779,  "NAD83(HARN) / Florida North",           "Florida North, USA",          "metre"),
    (2780,  "NAD83(HARN) / Georgia East",            "Georgia East, USA",           "metre"),
    (2781,  "NAD83(HARN) / Georgia West",            "Georgia West, USA",           "metre"),
    (2782,  "NAD83(HARN) / Hawaii zone 1",           "Hawaii zone 1, USA",          "metre"),
    (2783,  "NAD83(HARN) / Hawaii zone 2",           "Hawaii zone 2, USA",          "metre"),
    (2784,  "NAD83(HARN) / Hawaii zone 3",           "Hawaii zone 3, USA",          "metre"),
    (2785,  "NAD83(HARN) / Hawaii zone 4",           "Hawaii zone 4, USA",          "metre"),
    (2786,  "NAD83(HARN) / Hawaii zone 5",           "Hawaii zone 5, USA",          "metre"),
    (2787,  "NAD83(HARN) / Idaho East",              "Idaho East, USA",             "metre"),
    (2788,  "NAD83(HARN) / Idaho Central",           "Idaho Central, USA",          "metre"),
    (2789,  "NAD83(HARN) / Idaho West",              "Idaho West, USA",             "metre"),
    (2790,  "NAD83(HARN) / Illinois East",           "Illinois East, USA",          "metre"),
    (2791,  "NAD83(HARN) / Illinois West",           "Illinois West, USA",          "metre"),
    (2792,  "NAD83(HARN) / Indiana East",            "Indiana East, USA",           "metre"),
    (2793,  "NAD83(HARN) / Indiana West",            "Indiana West, USA",           "metre"),
    (2794,  "NAD83(HARN) / Iowa North",              "Iowa North, USA",             "metre"),
    (2795,  "NAD83(HARN) / Iowa South",              "Iowa South, USA",             "metre"),
    (2796,  "NAD83(HARN) / Kansas North",            "Kansas North, USA",           "metre"),
    (2797,  "NAD83(HARN) / Kansas South",            "Kansas South, USA",           "metre"),
    (2798,  "NAD83(HARN) / Kentucky North",          "Kentucky North, USA",         "metre"),
    (2799,  "NAD83(HARN) / Kentucky South",          "Kentucky South, USA",         "metre"),
    (2800,  "NAD83(HARN) / Louisiana North",         "Louisiana North, USA",        "metre"),
    (2801,  "NAD83(HARN) / Louisiana South",         "Louisiana South, USA",        "metre"),
    (2802,  "NAD83(HARN) / Maine East",              "Maine East, USA",             "metre"),
    (2803,  "NAD83(HARN) / Maine West",              "Maine West, USA",             "metre"),
    (2804,  "NAD83(HARN) / Maryland",                "Maryland, USA",               "metre"),
    (2805,  "NAD83(HARN) / Massachusetts Mainland",  "Massachusetts Mainland, USA", "metre"),
    (2806,  "NAD83(HARN) / Massachusetts Island",    "Massachusetts Island, USA",   "metre"),
    (2807,  "NAD83(HARN) / Michigan North",          "Michigan North, USA",         "metre"),
    (2808,  "NAD83(HARN) / Michigan Central",        "Michigan Central, USA",       "metre"),
    (2809,  "NAD83(HARN) / Michigan South",          "Michigan South, USA",         "metre"),
    (2810,  "NAD83(HARN) / Minnesota North",         "Minnesota North, USA",        "metre"),
    (2811,  "NAD83(HARN) / Minnesota Central",       "Minnesota Central, USA",      "metre"),
    (2812,  "NAD83(HARN) / Minnesota South",         "Minnesota South, USA",        "metre"),
    (2813,  "NAD83(HARN) / Mississippi East",        "Mississippi East, USA",       "metre"),
    (2814,  "NAD83(HARN) / Mississippi West",        "Mississippi West, USA",       "metre"),
    (2815,  "NAD83(HARN) / Missouri East",           "Missouri East, USA",          "metre"),
    (2816,  "NAD83(HARN) / Missouri Central",        "Missouri Central, USA",       "metre"),
    (2817,  "NAD83(HARN) / Missouri West",           "Missouri West, USA",          "metre"),
    (2818,  "NAD83(HARN) / Montana",                 "Montana, USA",                "metre"),
    (2819,  "NAD83(HARN) / Nebraska",                "Nebraska, USA",               "metre"),
    (2820,  "NAD83(HARN) / Nevada East",             "Nevada East, USA",            "metre"),
    (2821,  "NAD83(HARN) / Nevada Central",          "Nevada Central, USA",         "metre"),
    (2822,  "NAD83(HARN) / Nevada West",             "Nevada West, USA",            "metre"),
    (2823,  "NAD83(HARN) / New Hampshire",           "New Hampshire, USA",          "metre"),
    (2824,  "NAD83(HARN) / New Jersey",              "New Jersey, USA",             "metre"),
    (2825,  "NAD83(HARN) / New Mexico East",         "New Mexico East, USA",        "metre"),
    (2826,  "NAD83(HARN) / New Mexico Central",      "New Mexico Central, USA",     "metre"),
    (2827,  "NAD83(HARN) / New Mexico West",         "New Mexico West, USA",        "metre"),
    (2828,  "NAD83(HARN) / New York East",           "New York East, USA",          "metre"),
    (2829,  "NAD83(HARN) / New York Central",        "New York Central, USA",       "metre"),
    (2830,  "NAD83(HARN) / New York West",           "New York West, USA",          "metre"),
    (2831,  "NAD83(HARN) / New York Long Island",    "New York LI, USA",            "metre"),
    (2832,  "NAD83(HARN) / North Dakota North",      "North Dakota North, USA",     "metre"),
    (2833,  "NAD83(HARN) / North Dakota South",      "North Dakota South, USA",     "metre"),
    (2834,  "NAD83(HARN) / Ohio North",              "Ohio North, USA",             "metre"),
    (2835,  "NAD83(HARN) / Ohio South",              "Ohio South, USA",             "metre"),
    (2836,  "NAD83(HARN) / Oklahoma North",          "Oklahoma North, USA",         "metre"),
    (2837,  "NAD83(HARN) / Oklahoma South",          "Oklahoma South, USA",         "metre"),
    (2838,  "NAD83(HARN) / Oregon North",            "Oregon North, USA",           "metre"),
    (2839,  "NAD83(HARN) / Oregon South",            "Oregon South, USA",           "metre"),
    (2840,  "NAD83(HARN) / Rhode Island",            "Rhode Island, USA",           "metre"),
    (2841,  "NAD83(HARN) / South Dakota North",      "South Dakota North, USA",     "metre"),
    (2842,  "NAD83(HARN) / South Dakota South",      "South Dakota South, USA",     "metre"),
    (2843,  "NAD83(HARN) / Tennessee",               "Tennessee, USA",              "metre"),
    (2844,  "NAD83(HARN) / Texas North",             "Texas North, USA",            "metre"),
    (2845,  "NAD83(HARN) / Texas North Central",     "Texas North Central, USA",    "metre"),
    (2846,  "NAD83(HARN) / Texas Central",           "Texas Central, USA",          "metre"),
    (2847,  "NAD83(HARN) / Texas South Central",     "Texas South Central, USA",    "metre"),
    (2848,  "NAD83(HARN) / Texas South",             "Texas South, USA",            "metre"),
    (2849,  "NAD83(HARN) / Utah North",              "Utah North, USA",             "metre"),
    (2850,  "NAD83(HARN) / Utah Central",            "Utah Central, USA",           "metre"),
    (2851,  "NAD83(HARN) / Utah South",              "Utah South, USA",             "metre"),
    (2852,  "NAD83(HARN) / Vermont",                 "Vermont, USA",                "metre"),
    (2853,  "NAD83(HARN) / Virginia North",          "Virginia North, USA",         "metre"),
    (2854,  "NAD83(HARN) / Virginia South",          "Virginia South, USA",         "metre"),
    (2855,  "NAD83(HARN) / Washington North",        "Washington North, USA",       "metre"),
    (2856,  "NAD83(HARN) / Washington South",        "Washington South, USA",       "metre"),
    (2857,  "NAD83(HARN) / West Virginia North",     "West Virginia North, USA",    "metre"),
    (2858,  "NAD83(HARN) / West Virginia South",     "West Virginia South, USA",    "metre"),
    (2859,  "NAD83(HARN) / Wisconsin North",         "Wisconsin North, USA",        "metre"),
    (2860,  "NAD83(HARN) / Wisconsin Central",       "Wisconsin Central, USA",      "metre"),
    (2861,  "NAD83(HARN) / Wisconsin South",         "Wisconsin South, USA",        "metre"),
    (2862,  "NAD83(HARN) / Wyoming East",            "Wyoming East, USA",           "metre"),
    (2863,  "NAD83(HARN) / Wyoming East Central",    "Wyoming East Central, USA",   "metre"),
    (2864,  "NAD83(HARN) / Wyoming West Central",    "Wyoming West Central, USA",   "metre"),
    (2865,  "NAD83(HARN) / Wyoming West",            "Wyoming West, USA",           "metre"),
    (2866,  "NAD83(HARN) / Puerto Rico and Virgin Is.", "Puerto Rico and US Virgin Islands", "metre"),
    (3502,  "NAD83(NSRS2007) / Colorado Central (ftUS)", "Colorado Central, USA", "US survey foot"),
    (3338,  "NAD83 / Alaska zone 1",                "Alaska 1, USA",               "metre"),
    // Swiss
    (21781, "CH1903 / LV03",                        "Switzerland",                 "metre"),
    (2056,  "CH1903+ / LV95",                       "Switzerland",                 "metre"),
    // Japan
    (2443,  "JGD2000 / Japan Plane Rectangular CS I",   "Japan zone I",            "metre"),
    (2444,  "JGD2000 / Japan Plane Rectangular CS II",  "Japan zone II",           "metre"),
    (2445,  "JGD2000 / Japan Plane Rectangular CS III", "Japan zone III",          "metre"),
    (2446,  "JGD2000 / Japan Plane Rectangular CS IV",  "Japan zone IV",           "metre"),
    (2447,  "JGD2000 / Japan Plane Rectangular CS V",   "Japan zone V",            "metre"),
    (2448,  "JGD2000 / Japan Plane Rectangular CS VI",  "Japan zone VI",           "metre"),
    (2449,  "JGD2000 / Japan Plane Rectangular CS VII", "Japan zone VII",          "metre"),
    (2450,  "JGD2000 / Japan Plane Rectangular CS VIII", "Japan zone VIII",        "metre"),
    (2451,  "JGD2000 / Japan Plane Rectangular CS IX",  "Japan zone IX",           "metre"),
    (2452,  "JGD2000 / Japan Plane Rectangular CS X",   "Japan zone X",            "metre"),
    (2453,  "JGD2000 / Japan Plane Rectangular CS XI",  "Japan zone XI",           "metre"),
    (2454,  "JGD2000 / Japan Plane Rectangular CS XII", "Japan zone XII",          "metre"),
    (2455,  "JGD2000 / Japan Plane Rectangular CS XIII", "Japan zone XIII",        "metre"),
    (2456,  "JGD2000 / Japan Plane Rectangular CS XIV", "Japan zone XIV",          "metre"),
    (2457,  "JGD2000 / Japan Plane Rectangular CS XV",  "Japan zone XV",           "metre"),
    (2458,  "JGD2000 / Japan Plane Rectangular CS XVI", "Japan zone XVI",          "metre"),
    (2459,  "JGD2000 / Japan Plane Rectangular CS XVII", "Japan zone XVII",        "metre"),
    (2460,  "JGD2000 / Japan Plane Rectangular CS XVIII", "Japan zone XVIII",      "metre"),
    (2461,  "JGD2000 / Japan Plane Rectangular CS XIX", "Japan zone XIX",          "metre"),
    (6669,  "JGD2011 / Japan Plane Rectangular CS I",   "Japan zone I",            "metre"),
    (6670,  "JGD2011 / Japan Plane Rectangular CS II",  "Japan zone II",           "metre"),
    (6671,  "JGD2011 / Japan Plane Rectangular CS III", "Japan zone III",          "metre"),
    (6672,  "JGD2011 / Japan Plane Rectangular CS IV",  "Japan zone IV",           "metre"),
    (6673,  "JGD2011 / Japan Plane Rectangular CS V",   "Japan zone V",            "metre"),
    (6674,  "JGD2011 / Japan Plane Rectangular CS VI",  "Japan zone VI",           "metre"),
    (6675,  "JGD2011 / Japan Plane Rectangular CS VII", "Japan zone VII",          "metre"),
    (6676,  "JGD2011 / Japan Plane Rectangular CS VIII", "Japan zone VIII",        "metre"),
    (6677,  "JGD2011 / Japan Plane Rectangular CS IX",  "Japan zone IX",           "metre"),
    (6678,  "JGD2011 / Japan Plane Rectangular CS X",   "Japan zone X",            "metre"),
    (6679,  "JGD2011 / Japan Plane Rectangular CS XI",  "Japan zone XI",           "metre"),
    (6680,  "JGD2011 / Japan Plane Rectangular CS XII", "Japan zone XII",          "metre"),
    (6681,  "JGD2011 / Japan Plane Rectangular CS XIII", "Japan zone XIII",        "metre"),
    (6682,  "JGD2011 / Japan Plane Rectangular CS XIV", "Japan zone XIV",          "metre"),
    (6683,  "JGD2011 / Japan Plane Rectangular CS XV",  "Japan zone XV",           "metre"),
    (6684,  "JGD2011 / Japan Plane Rectangular CS XVI", "Japan zone XVI",          "metre"),
    (6685,  "JGD2011 / Japan Plane Rectangular CS XVII", "Japan zone XVII",        "metre"),
    (6686,  "JGD2011 / Japan Plane Rectangular CS XVIII", "Japan zone XVIII",      "metre"),
    (6687,  "JGD2011 / Japan Plane Rectangular CS XIX", "Japan zone XIX",          "metre"),
    (6688,  "JGD2011 / UTM zone 51N",                 "Japan",                       "metre"),
    (6689,  "JGD2011 / UTM zone 52N",                 "Japan",                       "metre"),
    (6690,  "JGD2011 / UTM zone 53N",                 "Japan",                       "metre"),
    (6691,  "JGD2011 / UTM zone 54N",                 "Japan",                       "metre"),
    (6692,  "JGD2011 / UTM zone 55N",                 "Japan",                       "metre"),
    (6707,  "RDN2008 / UTM zone 32N (N-E)",           "Italy",                       "metre"),
    (6708,  "RDN2008 / UTM zone 33N (N-E)",           "Italy",                       "metre"),
    (6709,  "RDN2008 / UTM zone 34N (N-E)",           "Italy",                       "metre"),
    (6732,  "GDA94 / MGA zone 41",                    "Australia region",            "metre"),
    (6733,  "GDA94 / MGA zone 42",                    "Australia region",            "metre"),
    (6734,  "GDA94 / MGA zone 43",                    "Australia region",            "metre"),
    (6735,  "GDA94 / MGA zone 44",                    "Australia region",            "metre"),
    (6736,  "GDA94 / MGA zone 46",                    "Australia region",            "metre"),
    (6737,  "GDA94 / MGA zone 47",                    "Australia region",            "metre"),
    (6738,  "GDA94 / MGA zone 59",                    "Australia region",            "metre"),
    (6784,  "NAD83(CORS96) / Oregon Baker zone (m)",  "Oregon, USA",                 "metre"),
    (6786,  "NAD83(2011) / Oregon Baker zone (m)",    "Oregon, USA",                 "metre"),
    (6788,  "NAD83(CORS96) / Oregon Bend-Klamath Falls zone (m)", "Oregon, USA", "metre"),
    (6790,  "NAD83(2011) / Oregon Bend-Klamath Falls zone (m)",   "Oregon, USA", "metre"),
    (6800,  "NAD83(CORS96) / Oregon Canyonville-Grants Pass zone (m)", "Oregon, USA", "metre"),
    (6802,  "NAD83(2011) / Oregon Canyonville-Grants Pass zone (m)",   "Oregon, USA", "metre"),
    (6812,  "NAD83(CORS96) / Oregon Cottage Grove-Canyonville zone (m)", "Oregon, USA", "metre"),
    (6814,  "NAD83(2011) / Oregon Cottage Grove-Canyonville zone (m)",   "Oregon, USA", "metre"),
    (6816,  "NAD83(CORS96) / Oregon Dufur-Madras zone (m)", "Oregon, USA", "metre"),
    (6818,  "NAD83(2011) / Oregon Dufur-Madras zone (m)",   "Oregon, USA", "metre"),
    (6820,  "NAD83(CORS96) / Oregon Eugene zone (m)",       "Oregon, USA", "metre"),
    (6822,  "NAD83(2011) / Oregon Eugene zone (m)",         "Oregon, USA", "metre"),
    (6824,  "NAD83(CORS96) / Oregon Grants Pass-Ashland zone (m)", "Oregon, USA", "metre"),
    (6826,  "NAD83(2011) / Oregon Grants Pass-Ashland zone (m)",   "Oregon, USA", "metre"),
    (6828,  "NAD83(CORS96) / Oregon Gresham-Warm Springs zone (m)", "Oregon, USA", "metre"),
    (6830,  "NAD83(2011) / Oregon Gresham-Warm Springs zone (m)",   "Oregon, USA", "metre"),
    (6832,  "NAD83(CORS96) / Oregon La Grande zone (m)",    "Oregon, USA", "metre"),
    (6834,  "NAD83(2011) / Oregon La Grande zone (m)",      "Oregon, USA", "metre"),
    (6836,  "NAD83(CORS96) / Oregon Ontario zone (m)",      "Oregon, USA", "metre"),
    (6838,  "NAD83(2011) / Oregon Ontario zone (m)",        "Oregon, USA", "metre"),
    (6844,  "NAD83(CORS96) / Oregon Pendleton zone (m)",    "Oregon, USA", "metre"),
    (6846,  "NAD83(2011) / Oregon Pendleton zone (m)",      "Oregon, USA", "metre"),
    (6848,  "NAD83(CORS96) / Oregon Pendleton-La Grande zone (m)", "Oregon, USA", "metre"),
    (6850,  "NAD83(2011) / Oregon Pendleton-La Grande zone (m)",   "Oregon, USA", "metre"),
    (6856,  "NAD83(CORS96) / Oregon Salem zone (m)",        "Oregon, USA", "metre"),
    (6858,  "NAD83(2011) / Oregon Salem zone (m)",          "Oregon, USA", "metre"),
    (6860,  "NAD83(CORS96) / Oregon Santiam Pass zone (m)", "Oregon, USA", "metre"),
    (6862,  "NAD83(2011) / Oregon Santiam Pass zone (m)",   "Oregon, USA", "metre"),
    (6870,  "ETRS89 / Albania TM 2010",                     "Albania",      "metre"),
    (6875,  "RDN2008 / Italy zone (N-E)",                  "Italy",        "metre"),
    (6876,  "RDN2008 / Zone 12 (N-E)",                     "Italy",        "metre"),
    (6915,  "South East Island 1943 / UTM zone 40N",       "South East Island", "metre"),
    (6927,  "SVY21 / Singapore TM",                        "Singapore",    "metre"),
    (6956,  "VN-2000 / TM-3 zone 481",                     "Vietnam",      "metre"),
    (6957,  "VN-2000 / TM-3 zone 482",                     "Vietnam",      "metre"),
    (7057,  "NAD83(2011) / IaRCS zone 1",                  "Iowa, USA",    "metre"),
    (7058,  "NAD83(2011) / IaRCS zone 2",                  "Iowa, USA",    "metre"),
    (7059,  "NAD83(2011) / IaRCS zone 3",                  "Iowa, USA",    "metre"),
    (7060,  "NAD83(2011) / IaRCS zone 4",                  "Iowa, USA",    "metre"),
    (7061,  "NAD83(2011) / IaRCS zone 5",                  "Iowa, USA",    "metre"),
    (7062,  "NAD83(2011) / IaRCS zone 6",                  "Iowa, USA",    "metre"),
    (7063,  "NAD83(2011) / IaRCS zone 7",                  "Iowa, USA",    "metre"),
    (7064,  "NAD83(2011) / IaRCS zone 8",                  "Iowa, USA",    "metre"),
    (7065,  "NAD83(2011) / IaRCS zone 9",                  "Iowa, USA",    "metre"),
    (7066,  "NAD83(2011) / IaRCS zone 10",                 "Iowa, USA",    "metre"),
    (7067,  "NAD83(2011) / IaRCS zone 11",                 "Iowa, USA",    "metre"),
    (7068,  "NAD83(2011) / IaRCS zone 12",                 "Iowa, USA",    "metre"),
    (7069,  "NAD83(2011) / IaRCS zone 13",                 "Iowa, USA",    "metre"),
    (7070,  "NAD83(2011) / IaRCS zone 14",                 "Iowa, USA",    "metre"),
    (7109,  "NAD83(2011) / RMTCRS St Mary (m)",            "Montana/Wyoming, USA", "metre"),
    (7110,  "NAD83(2011) / RMTCRS Blackfeet (m)",          "Montana, USA", "metre"),
    (7111,  "NAD83(2011) / RMTCRS Milk River (m)",         "Montana, USA", "metre"),
    (7112,  "NAD83(2011) / RMTCRS Fort Belknap (m)",       "Montana, USA", "metre"),
    (7113,  "NAD83(2011) / RMTCRS Fort Peck Assiniboine (m)", "Montana, USA", "metre"),
    (7114,  "NAD83(2011) / RMTCRS Fort Peck Sioux (m)",    "Montana, USA", "metre"),
    (7115,  "NAD83(2011) / RMTCRS Crow (m)",               "Montana, USA", "metre"),
    (7116,  "NAD83(2011) / RMTCRS Bobcat (m)",             "Montana, USA", "metre"),
    (7117,  "NAD83(2011) / RMTCRS Billings (m)",           "Montana, USA", "metre"),
    (7118,  "NAD83(2011) / RMTCRS Wind River (m)",         "Wyoming, USA", "metre"),
    (7131,  "NAD83(2011) / San Francisco CS13",            "California, USA", "metre"),
    (7257,  "NAD83(2011) / InGCS Adams (m)",               "Indiana, USA", "metre"),
    (7258,  "NAD83(2011) / InGCS Adams (ftUS)",            "Indiana, USA", "US survey foot"),
    (7259,  "NAD83(2011) / InGCS Allen (m)",               "Indiana, USA", "metre"),
    (7260,  "NAD83(2011) / InGCS Allen (ftUS)",            "Indiana, USA", "US survey foot"),
    (7261,  "NAD83(2011) / InGCS Bartholomew (m)",         "Indiana, USA", "metre"),
    (7262,  "NAD83(2011) / InGCS Bartholomew (ftUS)",      "Indiana, USA", "US survey foot"),
    (7263,  "NAD83(2011) / InGCS Benton (m)",              "Indiana, USA", "metre"),
    (7264,  "NAD83(2011) / InGCS Benton (ftUS)",           "Indiana, USA", "US survey foot"),
    (7265,  "NAD83(2011) / InGCS Blackford-Delaware (m)",  "Indiana, USA", "metre"),
    (7266,  "NAD83(2011) / InGCS Blackford-Delaware (ftUS)", "Indiana, USA", "US survey foot"),
    (7267,  "NAD83(2011) / InGCS Boone-Hendricks (m)",     "Indiana, USA", "metre"),
    (7268,  "NAD83(2011) / InGCS Boone-Hendricks (ftUS)",  "Indiana, USA", "US survey foot"),
    (7269,  "NAD83(2011) / InGCS Brown (m)",               "Indiana, USA", "metre"),
    (7270,  "NAD83(2011) / InGCS Brown (ftUS)",            "Indiana, USA", "US survey foot"),
    (7271,  "NAD83(2011) / InGCS Carroll (m)",             "Indiana, USA", "metre"),
    (7272,  "NAD83(2011) / InGCS Carroll (ftUS)",          "Indiana, USA", "US survey foot"),
    (7273,  "NAD83(2011) / InGCS Cass (m)",                "Indiana, USA", "metre"),
    (7274,  "NAD83(2011) / InGCS Cass (ftUS)",             "Indiana, USA", "US survey foot"),
    (7275,  "NAD83(2011) / InGCS Clark-Floyd-Scott (m)",   "Indiana, USA", "metre"),
    (7276,  "NAD83(2011) / InGCS Clark-Floyd-Scott (ftUS)", "Indiana, USA", "US survey foot"),
    (7277,  "NAD83(2011) / InGCS Clay (m)",                "Indiana, USA", "metre"),
    (7278,  "NAD83(2011) / InGCS Clay (ftUS)",             "Indiana, USA", "US survey foot"),
    (7279,  "NAD83(2011) / InGCS Clinton (m)",             "Indiana, USA", "metre"),
    (7280,  "NAD83(2011) / InGCS Clinton (ftUS)",          "Indiana, USA", "US survey foot"),
    (7281,  "NAD83(2011) / InGCS Crawford-Lawrence-Orange (m)", "Indiana, USA", "metre"),
    (7282,  "NAD83(2011) / InGCS Crawford-Lawrence-Orange (ftUS)", "Indiana, USA", "US survey foot"),
    (7283,  "NAD83(2011) / InGCS Daviess-Greene (m)",      "Indiana, USA", "metre"),
    (7284,  "NAD83(2011) / InGCS Daviess-Greene (ftUS)",   "Indiana, USA", "US survey foot"),
    (7285,  "NAD83(2011) / InGCS Dearborn-Ohio-Switzerland (m)", "Indiana, USA", "metre"),
    (7286,  "NAD83(2011) / InGCS Dearborn-Ohio-Switzerland (ftUS)", "Indiana, USA", "US survey foot"),
    (7287,  "NAD83(2011) / InGCS Decatur-Rush (m)",        "Indiana, USA", "metre"),
    (7288,  "NAD83(2011) / InGCS Decatur-Rush (ftUS)",     "Indiana, USA", "US survey foot"),
    (7289,  "NAD83(2011) / InGCS DeKalb (m)",              "Indiana, USA", "metre"),
    (7290,  "NAD83(2011) / InGCS DeKalb (ftUS)",           "Indiana, USA", "US survey foot"),
    (7291,  "NAD83(2011) / InGCS Dubois-Martin (m)",       "Indiana, USA", "metre"),
    (7292,  "NAD83(2011) / InGCS Dubois-Martin (ftUS)",    "Indiana, USA", "US survey foot"),
    (7293,  "NAD83(2011) / InGCS Elkhart-Kosciusko-Wabash (m)", "Indiana, USA", "metre"),
    (7294,  "NAD83(2011) / InGCS Elkhart-Kosciusko-Wabash (ftUS)", "Indiana, USA", "US survey foot"),
    (7295,  "NAD83(2011) / InGCS Fayette-Franklin-Union (m)", "Indiana, USA", "metre"),
    (7296,  "NAD83(2011) / InGCS Fayette-Franklin-Union (ftUS)", "Indiana, USA", "US survey foot"),
    (7297,  "NAD83(2011) / InGCS Fountain-Warren (m)",     "Indiana, USA", "metre"),
    (7298,  "NAD83(2011) / InGCS Fountain-Warren (ftUS)",  "Indiana, USA", "US survey foot"),
    (7299,  "NAD83(2011) / InGCS Fulton-Marshall-St. Joseph (m)", "Indiana, USA", "metre"),
    (7300,  "NAD83(2011) / InGCS Fulton-Marshall-St. Joseph (ftUS)", "Indiana, USA", "US survey foot"),
    (7301,  "NAD83(2011) / InGCS Gibson (m)",              "Indiana, USA", "metre"),
    (7302,  "NAD83(2011) / InGCS Gibson (ftUS)",           "Indiana, USA", "US survey foot"),
    (7303,  "NAD83(2011) / InGCS Grant (m)",               "Indiana, USA", "metre"),
    (7304,  "NAD83(2011) / InGCS Grant (ftUS)",            "Indiana, USA", "US survey foot"),
    (7305,  "NAD83(2011) / InGCS Hamilton-Tipton (m)",     "Indiana, USA", "metre"),
    (7306,  "NAD83(2011) / InGCS Hamilton-Tipton (ftUS)",  "Indiana, USA", "US survey foot"),
    (7307,  "NAD83(2011) / InGCS Hancock-Madison (m)",     "Indiana, USA", "metre"),
    (7309,  "NAD83(2011) / InGCS Harrison-Washington (m)", "Indiana, USA", "metre"),
    (7311,  "NAD83(2011) / InGCS Henry (m)",               "Indiana, USA", "metre"),
    (7313,  "NAD83(2011) / InGCS Howard-Miami (m)",        "Indiana, USA", "metre"),
    (7315,  "NAD83(2011) / InGCS Huntington-Whitley (m)",  "Indiana, USA", "metre"),
    (7317,  "NAD83(2011) / InGCS Jackson (m)",             "Indiana, USA", "metre"),
    (7319,  "NAD83(2011) / InGCS Jasper-Porter (m)",       "Indiana, USA", "metre"),
    (7321,  "NAD83(2011) / InGCS Jay (m)",                 "Indiana, USA", "metre"),
    (7323,  "NAD83(2011) / InGCS Jefferson (m)",           "Indiana, USA", "metre"),
    (7325,  "NAD83(2011) / InGCS Jennings (m)",            "Indiana, USA", "metre"),
    (7327,  "NAD83(2011) / InGCS Johnson-Marion (m)",      "Indiana, USA", "metre"),
    (7329,  "NAD83(2011) / InGCS Knox (m)",                "Indiana, USA", "metre"),
    (7331,  "NAD83(2011) / InGCS LaGrange-Noble (m)",      "Indiana, USA", "metre"),
    (7333,  "NAD83(2011) / InGCS Lake-Newton (m)",         "Indiana, USA", "metre"),
    (7335,  "NAD83(2011) / InGCS LaPorte-Pulaski-Starke (m)", "Indiana, USA", "metre"),
    (7337,  "NAD83(2011) / InGCS Monroe-Morgan (m)",       "Indiana, USA", "metre"),
    (7339,  "NAD83(2011) / InGCS Montgomery-Putnam (m)",   "Indiana, USA", "metre"),
    (7341,  "NAD83(2011) / InGCS Owen (m)",                "Indiana, USA", "metre"),
    (7343,  "NAD83(2011) / InGCS Parke-Vermillion (m)",    "Indiana, USA", "metre"),
    (7345,  "NAD83(2011) / InGCS Perry (m)",               "Indiana, USA", "metre"),
    (7347,  "NAD83(2011) / InGCS Pike-Warrick (m)",        "Indiana, USA", "metre"),
    (7349,  "NAD83(2011) / InGCS Posey (m)",               "Indiana, USA", "metre"),
    (7351,  "NAD83(2011) / InGCS Randolph-Wayne (m)",      "Indiana, USA", "metre"),
    (7353,  "NAD83(2011) / InGCS Ripley (m)",              "Indiana, USA", "metre"),
    (7355,  "NAD83(2011) / InGCS Shelby (m)",              "Indiana, USA", "metre"),
    // South Africa
    (22275, "Cape / Lo15",                          "South Africa (15°E)",         "metre"),
    (22277, "Cape / Lo17",                          "South Africa (17°E)",         "metre"),
    (22279, "Cape / Lo19",                          "South Africa (19°E)",         "metre"),
    (22281, "Cape / Lo21",                          "South Africa (21°E)",         "metre"),
    (22283, "Cape / Lo23",                          "South Africa (23°E)",         "metre"),
    (22285, "Cape / Lo25",                          "South Africa (25°E)",         "metre"),
    (22287, "Cape / Lo27",                          "South Africa (27°E)",         "metre"),
    (22289, "Cape / Lo29",                          "South Africa (29°E)",         "metre"),
    (22291, "Cape / Lo31",                          "South Africa (31°E)",         "metre"),
    (22293, "Cape / Lo33",                          "South Africa (33°E)",         "metre"),
    // ── GDA2020 MGA zones 49–56 ──────────────────────────────────────────
    (7849,  "GDA2020 / MGA zone 49",             "Australia",                  "metre"),
    (7850,  "GDA2020 / MGA zone 50",             "Australia",                  "metre"),
    (7851,  "GDA2020 / MGA zone 51",             "Australia",                  "metre"),
    (7852,  "GDA2020 / MGA zone 52",             "Australia",                  "metre"),
    (7853,  "GDA2020 / MGA zone 53",             "Australia",                  "metre"),
    (7854,  "GDA2020 / MGA zone 54",             "Australia",                  "metre"),
    (7855,  "GDA2020 / MGA zone 55",             "Australia",                  "metre"),
    (7856,  "GDA2020 / MGA zone 56",             "Australia",                  "metre"),
    // ── New geographic 2D ────────────────────────────────────────────────
    (4283,  "GDA94",                             "Australia",                  "degree"),
    (4148,  "Hartebeesthoek94",                  "South Africa",               "degree"),
    (4152,  "NAD83(HARN)",                       "United States",              "degree"),
    (4167,  "NZGD2000",                          "New Zealand",                "degree"),
    (4189,  "RGAF09",                            "French Caribbean",           "degree"),
    (4619,  "SIRGAS95",                          "Latin America",              "degree"),
    (4681,  "REGVEN",                            "Venezuela",                  "degree"),
    (4483,  "Mexico ITRF92",                     "Mexico",                     "degree"),
    (4624,  "RGFG95",                            "French Guiana",              "degree"),
    (4284,  "Pulkovo 1942",                      "Russia / Former USSR",       "degree"),
    (4322,  "WGS 72",                            "World",                      "degree"),
    (6318,  "NAD83(2011)",                       "United States",              "degree"),
    (4615,  "REGCAN95",                          "Canary Islands, Spain",      "degree"),
    // ── Legacy workflows parity block (4001–4063 selected) ─────────────
    (4001,  "Airy 1830",                         "World",                      "degree"),
    (4002,  "Airy Modified",                     "World",                      "degree"),
    (4003,  "Australian",                        "World",                      "degree"),
    (4004,  "Bessel 1841",                       "World",                      "degree"),
    (4005,  "Bessel Modified",                   "World",                      "degree"),
    (4006,  "Bessel Namibia",                    "Namibia",                    "degree"),
    (4007,  "Clarke 1858",                       "World",                      "degree"),
    (4008,  "Clarke 1866",                       "World",                      "degree"),
    (4009,  "Clarke 1866 Michigan",              "World",                      "degree"),
    (4010,  "Clarke 1880 Benoit",                "World",                      "degree"),
    (4011,  "Clarke 1880 IGN",                   "World",                      "degree"),
    (4012,  "Clarke 1880 RGS",                   "World",                      "degree"),
    (4013,  "Clarke 1880 Arc",                   "World",                      "degree"),
    (4014,  "Clarke 1880 SGA",                   "World",                      "degree"),
    (4015,  "Everest Adj 1937",                  "World",                      "degree"),
    (4016,  "Everest def 1967",                  "World",                      "degree"),
    (4018,  "Everest Modified",                  "World",                      "degree"),
    (4019,  "GRS 1980",                          "World",                      "degree"),
    (4020,  "Helmert 1906",                      "World",                      "degree"),
    (4021,  "Indonesian",                        "World",                      "degree"),
    (4022,  "International 1924",                "World",                      "degree"),
    (4023,  "MOLDREF99",                         "Moldova",                    "degree"),
    (4024,  "Krasovsky 1940",                    "World",                      "degree"),
    (4025,  "NWL 9D",                            "World",                      "degree"),
    (4026,  "MOLDREF99 / Moldova TM",            "Moldova",                    "metre"),
    (4027,  "Plessis 1817",                      "World",                      "degree"),
    (4028,  "Struve 1860",                       "World",                      "degree"),
    (4029,  "War Office",                        "World",                      "degree"),
    (4031,  "GEM 10C",                           "World",                      "degree"),
    (4032,  "OSU 86F",                           "World",                      "degree"),
    (4033,  "OSU 91A",                           "World",                      "degree"),
    (4034,  "Clarke 1880",                       "World",                      "degree"),
    (4035,  "Sphere",                            "World",                      "degree"),
    (4036,  "GRS 1967",                          "World",                      "degree"),
    (4037,  "WGS 84 / TMzn35N",                  "World",                      "metre"),
    (4038,  "WGS 84 / TMzn36N",                  "World",                      "metre"),
    (4044,  "Everest def 1962",                  "World",                      "degree"),
    (4045,  "Everest def 1975",                  "World",                      "degree"),
    (4046,  "RGRDC 2005",                        "Democratic Republic of the Congo", "degree"),
    (4047,  "Sphere GRS 1980 Authalic",          "World",                      "degree"),
    (4048,  "RGRDC 2005 / Congo TM zone 12",     "Democratic Republic of the Congo", "metre"),
    (4049,  "RGRDC 2005 / Congo TM zone 14",     "Democratic Republic of the Congo", "metre"),
    (4050,  "RGRDC 2005 / Congo TM zone 16",     "Democratic Republic of the Congo", "metre"),
    (4051,  "RGRDC 2005 / Congo TM zone 18",     "Democratic Republic of the Congo", "metre"),
    (4052,  "Sphere Clarke 1866 Authalic",       "World",                      "degree"),
    (4053,  "Sphere International 1924 Authalic", "World",                     "degree"),
    (4054,  "Hughes 1980",                       "World",                      "degree"),
    (4055,  "WGS 84 Major Auxiliary Sphere",     "World",                      "degree"),
    (4056,  "RGRDC 2005 / Congo TM zone 20",     "Democratic Republic of the Congo", "metre"),
    (4057,  "RGRDC 2005 / Congo TM zone 22",     "Democratic Republic of the Congo", "metre"),
    (4058,  "RGRDC 2005 / Congo TM zone 24",     "Democratic Republic of the Congo", "metre"),
    (4059,  "RGRDC 2005 / Congo TM zone 26",     "Democratic Republic of the Congo", "metre"),
    (4060,  "RGRDC 2005 / Congo TM zone 28",     "Democratic Republic of the Congo", "metre"),
    (4061,  "RGRDC 2005 / UTM zone 33S",         "Democratic Republic of the Congo", "metre"),
    (4062,  "RGRDC 2005 / UTM zone 34S",         "Democratic Republic of the Congo", "metre"),
    (4063,  "RGRDC 2005 / UTM zone 35S",         "Democratic Republic of the Congo", "metre"),
    // ── SWEREF99 local TM zones ──────────────────────────────────────────
    (3007,  "SWEREF99 12 00",                    "Sweden",                     "metre"),
    (3008,  "SWEREF99 13 30",                    "Sweden",                     "metre"),
    (3009,  "SWEREF99 15 00",                    "Sweden",                     "metre"),
    (3010,  "SWEREF99 16 30",                    "Sweden",                     "metre"),
    (3011,  "SWEREF99 18 00",                    "Sweden",                     "metre"),
    (3012,  "SWEREF99 14 15",                    "Sweden",                     "metre"),
    (3013,  "SWEREF99 15 45",                    "Sweden",                     "metre"),
    (3014,  "SWEREF99 17 15",                    "Sweden",                     "metre"),
    // ── Poland CS2000 / CS92 ─────────────────────────────────────────────
    (2176,  "ETRS89 / Poland CS2000 zone 5",     "Poland",                     "metre"),
    (2177,  "ETRS89 / Poland CS2000 zone 6",     "Poland",                     "metre"),
    (2178,  "ETRS89 / Poland CS2000 zone 7",     "Poland",                     "metre"),
    (2179,  "ETRS89 / Poland CS2000 zone 8",     "Poland",                     "metre"),
    (2180,  "ETRS89 / Poland CS92",              "Poland",                     "metre"),
    // ── European national grids ──────────────────────────────────────────
    (2100,  "GGRS87 / Greek Grid",               "Greece",                     "metre"),
    (23700, "HD72 / EOV",                        "Hungary",                    "metre"),
    (31700, "Dealul Piscului 1970 / Stereo 70",  "Romania",                    "metre"),
    (3763,  "ETRS89 / Portugal TM06",            "Portugal",                   "metre"),
    (3765,  "HTRS96 / Croatia TM",               "Croatia",                    "metre"),
    (3301,  "ETRS89 / Estonian CRS L-EST97",     "Estonia",                    "metre"),
    (5243,  "ETRS89 / LCC Germany (N)",          "Germany",                    "metre"),
    // ── Middle East / Asia-Pacific ───────────────────────────────────────
    (2039,  "Israel 1993 / Israeli TM Grid",     "Israel",                     "metre"),
    (3414,  "SVY21 / Singapore TM",              "Singapore",                  "metre"),
    (2326,  "Hong Kong 1980 Grid",               "Hong Kong",                  "metre"),
    // ── Americas ─────────────────────────────────────────────────────────
    (3347,  "NAD83 / Statistics Canada Lambert", "Canada",                     "metre"),
    (3978,  "NAD83 / Canada Atlas Lambert",      "Canada",                     "metre"),
    (3174,  "NAD83 / Great Lakes and St Lawrence Albers", "North America",     "metre"),
    (6350,  "NAD83(2011) / CONUS Albers",        "CONUS, USA",                 "metre"),
    // ── Australia additional ─────────────────────────────────────────────
    (3111,  "GDA94 / VicGrid",                   "Victoria, Australia",        "metre"),
    (3308,  "GDA94 / NSW Lambert",               "New South Wales, Australia", "metre"),
    // ── Additional EPSG batch ────────────────────────────────────────────
    (7846,  "GDA2020 / MGA zone 46",             "Australia",                  "metre"),
    (7847,  "GDA2020 / MGA zone 47",             "Australia",                  "metre"),
    (7848,  "GDA2020 / MGA zone 48",             "Australia",                  "metre"),
    (3812,  "ETRS89 / Belgian Lambert 2008",     "Belgium",                    "metre"),
    (31256, "MGI / Austria GK East",             "Austria",                    "metre"),
    (31257, "MGI / Austria GK M28",              "Austria",                    "metre"),
    (31258, "MGI / Austria GK M31",              "Austria",                    "metre"),
    (31287, "MGI / Austria Lambert",             "Austria",                    "metre"),
    (5179,  "KGD2002 / Unified CS",              "Korea",                      "metre"),
    (5181,  "KGD2002 / Central Belt",            "Korea",                      "metre"),
    (5182,  "KGD2002 / Central Belt Jeju",       "Korea",                      "metre"),
    (5186,  "KGD2002 / Central Belt 2010",       "Korea",                      "metre"),
    (5187,  "KGD2002 / East Belt 2010",          "Korea",                      "metre"),
    (3825,  "TWD97 / TM2 zone 119",              "Taiwan",                     "metre"),
    (3826,  "TWD97 / TM2 zone 121",              "Taiwan",                     "metre"),
    (3112,  "GDA94 / Geoscience Australia Lambert", "Australia",               "metre"),
    (3005,  "NAD83 / BC Albers",                 "British Columbia, Canada",   "metre"),
    (3015,  "SWEREF99 18 45",                    "Sweden",                     "metre"),
    (3767,  "HTRS96 / UTM zone 33N",             "Croatia",                    "metre"),
    (2046,  "Hartebeesthoek94 / Lo15",           "South Africa",               "metre"),
    (2047,  "Hartebeesthoek94 / Lo17",           "South Africa",               "metre"),
    // ── Additional EPSG batch II ────────────────────────────────────────
    (7843,  "GDA2020",                           "Australia",                  "degree"),
    (7845,  "GDA2020 / GA LCC",                  "Australia",                  "metre"),
    (5513,  "S-JTSK / Krovak",                   "Czech Republic and Slovakia", "metre"),
    (2065,  "S-JTSK (Ferro) / Krovak",           "Czech Republic and Slovakia", "metre"),
    (31254, "MGI / Austria GK West",             "Austria",                    "metre"),
    (31255, "MGI / Austria GK Central",          "Austria",                    "metre"),
    (31265, "MGI / 3-degree Gauss zone 5",       "Austria",                    "metre"),
    (31266, "MGI / 3-degree Gauss zone 6",       "Austria",                    "metre"),
    (31267, "MGI / 3-degree Gauss zone 7",       "Austria",                    "metre"),
    (3766,  "HTRS96 / Croatia LCC",              "Croatia",                    "metre"),
    (2048,  "Hartebeesthoek94 / Lo19",           "South Africa",               "metre"),
    (2049,  "Hartebeesthoek94 / Lo21",           "South Africa",               "metre"),
    (2050,  "Hartebeesthoek94 / Lo23",           "South Africa",               "metre"),
    (2051,  "Hartebeesthoek94 / Lo25",           "South Africa",               "metre"),
    (2052,  "Hartebeesthoek94 / Lo27",           "South Africa",               "metre"),
    (2053,  "Hartebeesthoek94 / Lo29",           "South Africa",               "metre"),
    (2054,  "Hartebeesthoek94 / Lo31",           "South Africa",               "metre"),
    (2055,  "Hartebeesthoek94 / Lo33",           "South Africa",               "metre"),
    (2040,  "Locodjo 1965 / UTM zone 30N",       "Cote d'Ivoire",              "metre"),
    (2041,  "Abidjan 1987 / UTM zone 30N",       "Cote d'Ivoire",              "metre"),
    (2042,  "Locodjo 1965 / UTM zone 29N",       "Cote d'Ivoire",              "metre"),
    (2043,  "Abidjan 1987 / UTM zone 29N",       "Cote d'Ivoire",              "metre"),
    (2057,  "Rassadiran / Nakhl-e Taqi",         "Iran",                       "metre"),
    (2058,  "ED50(ED77) / UTM zone 38N",         "Middle East",                "metre"),
    (2059,  "ED50(ED77) / UTM zone 39N",         "Middle East",                "metre"),
    (2060,  "ED50(ED77) / UTM zone 40N",         "Middle East",                "metre"),
    (2061,  "ED50(ED77) / UTM zone 41N",         "Middle East",                "metre"),
    (2063,  "Dabola 1981 / UTM zone 28N",        "Guinea",                     "metre"),
    (2064,  "Dabola 1981 / UTM zone 29N",        "Guinea",                     "metre"),
    (2067,  "Naparima 1955 / UTM zone 20N",      "Trinidad and Tobago",        "metre"),
    (2068,  "ELD79 / Libya zone 5",              "Libya",                      "metre"),
    (2069,  "ELD79 / Libya zone 6",              "Libya",                      "metre"),
    (2070,  "ELD79 / Libya zone 7",              "Libya",                      "metre"),
    (2071,  "ELD79 / Libya zone 8",              "Libya",                      "metre"),
    (2072,  "ELD79 / Libya zone 9",              "Libya",                      "metre"),
    (2073,  "ELD79 / Libya zone 10",             "Libya",                      "metre"),
    (2074,  "ELD79 / Libya zone 11",             "Libya",                      "metre"),
    (2075,  "ELD79 / Libya zone 12",             "Libya",                      "metre"),
    (2076,  "ELD79 / Libya zone 13",             "Libya",                      "metre"),
    (2077,  "ELD79 / UTM zone 32N",              "Libya",                      "metre"),
    (2078,  "ELD79 / UTM zone 33N",              "Libya",                      "metre"),
    (2079,  "ELD79 / UTM zone 34N",              "Libya",                      "metre"),
    (2080,  "ELD79 / UTM zone 35N",              "Libya",                      "metre"),
    (2085,  "NAD27 / Cuba Norte",                "Cuba",                       "metre"),
    (2086,  "NAD27 / Cuba Sur",                  "Cuba",                       "metre"),
    (2087,  "ELD79 / TM 12 NE",                  "Libya",                      "metre"),
    (2088,  "Carthage / TM 11 NE",               "Tunisia",                    "metre"),
    (2089,  "Yemen NGN96 / UTM zone 38N",        "Yemen",                      "metre"),
    (2090,  "Yemen NGN96 / UTM zone 39N",        "Yemen",                      "metre"),
    (2091,  "South Yemen / GK zone 8",           "Yemen",                      "metre"),
    (2092,  "South Yemen / GK zone 9",           "Yemen",                      "metre"),
    (2093,  "Hanoi 1972 / GK 106 NE",            "Vietnam",                    "metre"),
    (2094,  "WGS 72BE / TM 106 NE",              "Vietnam",                    "metre"),
    (2095,  "Bissau / UTM zone 28N",             "Guinea-Bissau",              "metre"),
    (2096,  "Korean 1985 / East Belt",           "Korea",                      "metre"),
    (2097,  "Korean 1985 / Central Belt",        "Korea",                      "metre"),
    (2098,  "Korean 1985 / West Belt",           "Korea",                      "metre"),
    (2105,  "NZGD2000 / Mount Eden Circuit",     "New Zealand",                "metre"),
    (2106,  "NZGD2000 / Bay of Plenty Circuit",  "New Zealand",                "metre"),
    (2107,  "NZGD2000 / Poverty Bay Circuit",    "New Zealand",                "metre"),
    (2108,  "NZGD2000 / Hawkes Bay Circuit",     "New Zealand",                "metre"),
    (2109,  "NZGD2000 / Taranaki Circuit",       "New Zealand",                "metre"),
    (2110,  "NZGD2000 / Tuhirangi Circuit",      "New Zealand",                "metre"),
    (2111,  "NZGD2000 / Wanganui Circuit",       "New Zealand",                "metre"),
    (2112,  "NZGD2000 / Wairarapa Circuit",      "New Zealand",                "metre"),
    (2113,  "NZGD2000 / Wellington Circuit",     "New Zealand",                "metre"),
    (2114,  "NZGD2000 / Collingwood Circuit",    "New Zealand",                "metre"),
    (2115,  "NZGD2000 / Nelson Circuit",         "New Zealand",                "metre"),
    (2116,  "NZGD2000 / Karamea Circuit",        "New Zealand",                "metre"),
    (2117,  "NZGD2000 / Buller Circuit",         "New Zealand",                "metre"),
    (2118,  "NZGD2000 / Grey Circuit",           "New Zealand",                "metre"),
    (2119,  "NZGD2000 / Amuri Circuit",          "New Zealand",                "metre"),
    (2120,  "NZGD2000 / Marlborough Circuit",    "New Zealand",                "metre"),
    (2121,  "NZGD2000 / Hokitika Circuit",       "New Zealand",                "metre"),
    (2122,  "NZGD2000 / Okarito Circuit",        "New Zealand",                "metre"),
    (2123,  "NZGD2000 / Jacksons Bay Circuit",   "New Zealand",                "metre"),
    (2124,  "NZGD2000 / Mount Pleasant Circuit", "New Zealand",                "metre"),
    (2125,  "NZGD2000 / Gawler Circuit",         "New Zealand",                "metre"),
    (2126,  "NZGD2000 / Timaru Circuit",         "New Zealand",                "metre"),
    (2127,  "NZGD2000 / Lindis Peak Circuit",    "New Zealand",                "metre"),
    (2128,  "NZGD2000 / Mount Nicholas Circuit", "New Zealand",                "metre"),
    (2129,  "NZGD2000 / Mount York Circuit",     "New Zealand",                "metre"),
    (2130,  "NZGD2000 / Observation Point Circuit", "New Zealand",             "metre"),
    (2131,  "NZGD2000 / North Taieri Circuit",   "New Zealand",                "metre"),
    (2132,  "NZGD2000 / Bluff Circuit",          "New Zealand",                "metre"),
    (2133,  "NZGD2000 / UTM zone 58S",           "New Zealand",                "metre"),
    (2134,  "NZGD2000 / UTM zone 59S",           "New Zealand",                "metre"),
    (2135,  "NZGD2000 / UTM zone 60S",           "New Zealand",                "metre"),
    (2136,  "Accra / Ghana Grid",                "Ghana",                      "foot"),
    (2137,  "Accra / TM 1 NW",                   "Ghana",                      "metre"),
    (2138,  "NAD27(CGQ77) / Quebec Lambert",     "Canada",                     "metre"),
    (2148,  "NAD83(CSRS) / UTM zone 21N",        "Canada",                     "metre"),
    (2149,  "NAD83(CSRS) / UTM zone 18N",        "Canada",                     "metre"),
    (2150,  "NAD83(CSRS) / UTM zone 17N",        "Canada",                     "metre"),
    (2151,  "NAD83(CSRS) / UTM zone 13N",        "Canada",                     "metre"),
    (2152,  "NAD83(CSRS) / UTM zone 12N",        "Canada",                     "metre"),
    (2153,  "NAD83(CSRS) / UTM zone 11N",        "Canada",                     "metre"),
    (2158,  "IRENET95 / UTM zone 29N",           "Ireland",                    "metre"),
    (2159,  "Sierra Leone 1924 / New Colony Grid", "Sierra Leone",             "foot"),
    (2160,  "Sierra Leone 1924 / New War Office Grid", "Sierra Leone",         "foot"),
    (2161,  "Sierra Leone 1968 / UTM zone 28N",  "Sierra Leone",               "metre"),
    (2162,  "Sierra Leone 1968 / UTM zone 29N",  "Sierra Leone",               "metre"),
    (2164,  "Locodjo 1965 / TM 5 NW",            "Cote d'Ivoire",              "metre"),
    (2165,  "Abidjan 1987 / TM 5 NW",            "Cote d'Ivoire",              "metre"),
    (2166,  "Pulkovo 1942(83) / 3-degree GK zone 3", "Central Europe",         "metre"),
    (2167,  "Pulkovo 1942(83) / 3-degree GK zone 4", "Central Europe",         "metre"),
    (2168,  "Pulkovo 1942(83) / 3-degree GK zone 5", "Central Europe",         "metre"),
    (2169,  "Luxembourg 1930 / Gauss",           "Luxembourg",                 "metre"),
    (2170,  "MGI / Slovenia Grid",               "Slovenia",                   "metre"),
    (2172,  "Pulkovo 1942 Adj 1958 / Poland zone II", "Poland",                "metre"),
    (2173,  "Pulkovo 1942 Adj 1958 / Poland zone III", "Poland",               "metre"),
    (2174,  "Pulkovo 1942 Adj 1958 / Poland zone IV", "Poland",                "metre"),
    (2175,  "Pulkovo 1942 Adj 1958 / Poland zone V", "Poland",                 "metre"),
    (2188,  "Azores Occidental 1939 / UTM zone 25N", "Azores",                 "metre"),
    (2189,  "Azores Central 1948 / UTM zone 26N", "Azores",                    "metre"),
    (2190,  "Azores Oriental 1940 / UTM zone 26N", "Azores",                   "metre"),
    (2191,  "Madeira 1936 / UTM zone 28N",         "Madeira",                  "metre"),
    (2192,  "ED50 / France EuroLambert",           "France",                   "metre"),
    (2195,  "NAD83(HARN) / UTM zone 2S",           "Pacific",                  "metre"),
    (2196,  "ETRS89 / Kp2000 Jutland",             "Denmark",                  "metre"),
    (2197,  "ETRS89 / Kp2000 Zealand",             "Denmark",                  "metre"),
    (2198,  "ETRS89 / Kp2000 Bornholm",            "Denmark",                  "metre"),
    (2200,  "ATS77 / New Brunswick Stereographic", "Canada",                   "metre"),
    (2201,  "REGVEN / UTM zone 18N",               "Venezuela",                "metre"),
    (2202,  "REGVEN / UTM zone 19N",               "Venezuela",                "metre"),
    (2203,  "REGVEN / UTM zone 20N",               "Venezuela",                "metre"),
    (2204,  "NAD27 / Tennessee (FIPS 4100) ftUS",  "USA",                      "US survey foot"),
    (2205,  "NAD83 / Kentucky North (FIPS 1601)",  "USA",                      "metre"),
    (2206,  "ED50 / 3-degree GK zone 9",           "Europe",                   "metre"),
    (2207,  "ED50 / 3-degree GK zone 10",          "Europe",                   "metre"),
    (2208,  "ED50 / 3-degree GK zone 11",          "Europe",                   "metre"),
    (2209,  "ED50 / 3-degree GK zone 12",          "Europe",                   "metre"),
    (2210,  "ED50 / 3-degree GK zone 13",          "Europe",                   "metre"),
    (2211,  "ED50 / 3-degree GK zone 14",          "Europe",                   "metre"),
    (2212,  "ED50 / 3-degree GK zone 15",          "Europe",                   "metre"),
    (2213,  "ETRS89 / TM 30 NE",                   "Europe",                   "metre"),
    (2214,  "Douala 1948 / AEF West",              "Cameroon",                 "metre"),
    (2215,  "Manoca 1962 / UTM zone 32N",          "Cameroon",                 "metre"),
    (2216,  "Qornoq 1927 / UTM zone 22N",          "Greenland",                "metre"),
    (2217,  "Qornoq 1927 / UTM zone 23N",          "Greenland",                "metre"),
    (2219,  "ATS77 / UTM zone 19N",                "Canada",                   "metre"),
    (2220,  "ATS77 / UTM zone 20N",                "Canada",                   "metre"),
    (2222,  "NAD83 / Arizona East (FIPS 0201) ft", "USA",                      "foot"),
    (2223,  "NAD83 / Arizona Central (FIPS 0202) ft", "USA",                   "foot"),
    (2224,  "NAD83 / Arizona West (FIPS 0203) ft", "USA",                      "foot"),
    (2225,  "NAD83 / California zone 1 (ftUS)",    "California, USA",          "US survey foot"),
    (2226,  "NAD83 / California zone 2 (ftUS)",    "California, USA",          "US survey foot"),
    (2228,  "NAD83 / California zone 4 (ftUS)",    "California, USA",          "US survey foot"),
    (2252,  "NAD83 / Michigan Central (FIPS 2112) ft", "USA",                  "foot"),
    (2253,  "NAD83 / Michigan South (FIPS 2113) ft", "USA",                    "foot"),
    (2254,  "NAD83 / Mississippi East (FIPS 2301) ftUS", "USA",                "US survey foot"),
    (2255,  "NAD83 / Mississippi West (FIPS 2302) ftUS", "USA",                "US survey foot"),
    (2256,  "NAD83 / Montana (FIPS 2500) ft",      "USA",                      "foot"),
    (2257,  "NAD83 / New Mexico East (FIPS 3001) ftUS", "USA",                 "US survey foot"),
    (2258,  "NAD83 / New Mexico Central (FIPS 3002) ftUS", "USA",              "US survey foot"),
    (2259,  "NAD83 / New Mexico West (FIPS 3003) ftUS", "USA",                 "US survey foot"),
    (2260,  "NAD83 / New York East (FIPS 3101) ftUS", "USA",                   "US survey foot"),
    (2261,  "NAD83 / New York Central (FIPS 3102) ftUS", "USA",                "US survey foot"),
    (2262,  "NAD83 / New York West (FIPS 3103) ftUS", "USA",                   "US survey foot"),
    (2264,  "NAD83 / North Carolina (FIPS 3200) ftUS", "USA",                  "US survey foot"),
    (2265,  "NAD83 / North Dakota North (FIPS 3301) ft", "USA",                "foot"),
    (2266,  "NAD83 / North Dakota South (FIPS 3302) ft", "USA",                "foot"),
    (2267,  "NAD83 / Oklahoma North (FIPS 3501) ftUS", "USA",                  "US survey foot"),
    (2268,  "NAD83 / Oklahoma South (FIPS 3502) ftUS", "USA",                  "US survey foot"),
    (2269,  "NAD83 / Oregon North (FIPS 3601) ft", "USA",                      "foot"),
    (2270,  "NAD83 / Oregon South (FIPS 3602) ft", "USA",                      "foot"),
    (2271,  "NAD83 / Pennsylvania North (FIPS 3701) ftUS", "USA",              "US survey foot"),
    (2274,  "NAD83 / Tennessee (FIPS 4100) ftUS",  "USA",                      "US survey foot"),
    (2275,  "NAD83 / Texas North (FIPS 4201) ftUS", "USA",                     "US survey foot"),
    (2276,  "NAD83 / Texas North Central (FIPS 4202) ftUS", "USA",             "US survey foot"),
    (2277,  "NAD83 / Texas Central (FIPS 4203) ftUS", "USA",                   "US survey foot"),
    (2278,  "NAD83 / Texas South Central (FIPS 4204) ftUS", "USA",             "US survey foot"),
    (2279,  "NAD83 / Texas South (FIPS 4205) ftUS", "USA",                     "US survey foot"),
    (2280,  "NAD83 / Utah North (FIPS 4301) ft",   "USA",                      "foot"),
    (2281,  "NAD83 / Utah Central (FIPS 4302) ft", "USA",                      "foot"),
    (2282,  "NAD83 / Utah South (FIPS 4303) ft",   "USA",                      "foot"),
    (2287,  "NAD83 / Wisconsin North (FIPS 4801) ftUS", "USA",                 "US survey foot"),
    (2288,  "NAD83 / Wisconsin Central (FIPS 4802) ftUS", "USA",               "US survey foot"),
    (2289,  "NAD83 / Wisconsin South (FIPS 4803) ftUS", "USA",                 "US survey foot"),
    (2290,  "ATS77 / Prince Edward Island Stereographic", "Canada",            "metre"),
    (2291,  "NAD83(CSRS) / Prince Edward Island", "Canada",                    "metre"),
    (2292,  "NAD83(CSRS) / Prince Edward Island", "Canada",                    "metre"),
    (2294,  "ATS77 / MTM Nova Scotia zone 4",      "Canada",                   "metre"),
    (2295,  "ATS77 / MTM Nova Scotia zone 5",      "Canada",                   "metre"),
    (2308,  "Batavia / TM 109 SE",                 "Indonesia",                "metre"),
    (2309,  "WGS 84 / TM 116 SE",                  "Indonesia",                "metre"),
    (2310,  "WGS 84 / TM 132 SE",                  "Indonesia",                "metre"),
    (2311,  "WGS 84 / TM 6 NE",                    "Norway",                   "metre"),
    (2312,  "Garoua / UTM zone 33N",               "Cameroon",                 "metre"),
    (2313,  "Kousseri / UTM zone 33N",             "Cameroon",                 "metre"),
    (2314,  "Trinidad 1903 / Trinidad Grid",       "Trinidad and Tobago",      "foot"),
    (2315,  "Campo Inchauspe / UTM zone 19S",      "Argentina",                "metre"),
    (2316,  "Campo Inchauspe / UTM zone 20S",      "Argentina",                "metre"),
    (2317,  "PSAD56 / ICN Regional",               "South America",            "metre"),
    (2318,  "Ain el Abd 1970 / Aramco Lambert",    "Saudi Arabia",             "metre"),
    (2319,  "ED50 / TM27",                         "Europe",                   "metre"),
    (2320,  "ED50 / TM30",                         "Europe",                   "metre"),
    (2321,  "ED50 / TM33",                         "Europe",                   "metre"),
    (2322,  "ED50 / TM36",                         "Europe",                   "metre"),
    (2323,  "ED50 / TM39",                         "Europe",                   "metre"),
    (2324,  "ED50 / TM42",                         "Europe",                   "metre"),
    (2325,  "ED50 / TM45",                         "Europe",                   "metre"),
    (2327,  "Xian 1980 / GK zone 13",              "China",                    "metre"),
    (2328,  "Xian 1980 / GK zone 14",              "China",                    "metre"),
    (2329,  "Xian 1980 / GK zone 15",              "China",                    "metre"),
    (2330,  "Xian 1980 / GK zone 16",              "China",                    "metre"),
    (2331,  "Xian 1980 / GK zone 17",              "China",                    "metre"),
    (2332,  "Xian 1980 / GK zone 18",              "China",                    "metre"),
    (2333,  "Xian 1980 / GK zone 19",              "China",                    "metre"),
    (2397,  "Pulkovo 1942(83) / 3-degree GK zone 3", "Central Europe",         "metre"),
    (2398,  "Pulkovo 1942(83) / 3-degree GK zone 4", "Central Europe",         "metre"),
    (2399,  "Pulkovo 1942(83) / 3-degree GK zone 5", "Central Europe",         "metre"),
    (31275, "MGI / Balkans zone 5",              "Balkans",                    "metre"),
    (31276, "MGI / Balkans zone 6",              "Balkans",                    "metre"),
];

fn get_info(code: u32) -> Option<EpsgInfo> {
    // Check named entries first
    if let Some(&(c, name, area, unit)) = NAMED_ENTRIES.iter().find(|e| e.0 == code) {
        return Some(EpsgInfo { code: c, name, area_of_use: area, unit });
    }
    // Dynamic UTM ranges
    if (32601..=32660).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 84 / UTM (northern hemisphere)",
            area_of_use: "World — UTM north",
            unit: "metre",
        });
    }
    if (32701..=32760).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 84 / UTM (southern hemisphere)",
            area_of_use: "World — UTM south",
            unit: "metre",
        });
    }
    if (32201..=32260).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 72 / UTM (northern hemisphere)",
            area_of_use: "World — UTM north",
            unit: "metre",
        });
    }
    if (32301..=32360).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 72 / UTM (southern hemisphere)",
            area_of_use: "World — UTM south",
            unit: "metre",
        });
    }
    if (32401..=32460).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 72BE / UTM (northern hemisphere)",
            area_of_use: "World — UTM north",
            unit: "metre",
        });
    }
    if (32501..=32560).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "WGS 72BE / UTM (southern hemisphere)",
            area_of_use: "World — UTM south",
            unit: "metre",
        });
    }
    if (2494..=2522).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1942 / Gauss-Kruger (CM)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (2463..=2491).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1995 / Gauss-Kruger (CM)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (2523..=2581).contains(&code) && code != 2550 {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1942 / 3-degree Gauss-Kruger (zone)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (2582..=2640).contains(&code) && code != 2600 {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1942 / 3-degree Gauss-Kruger (CM)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (2641..=2698).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1995 / 3-degree Gauss-Kruger (zone)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (2699..=2758).contains(&code) && code != 2736 && code != 2737 {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1995 / 3-degree Gauss-Kruger (CM)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (20004..=20032).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1995 / Gauss-Kruger (zone)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if (28404..=28432).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo 1942 / Gauss-Kruger (zone)",
            area_of_use: "Former USSR",
            unit: "metre",
        });
    }
    if [
        3329u32, 3330, 3331, 3332, 3333, 3334, 3335,
        4417, 4434,
        5631, 5663, 5664, 5665,
        5670, 5671, 5672, 5673, 5674, 5675,
    ]
    .contains(&code)
    {
        return Some(EpsgInfo {
            code,
            name: "Pulkovo adjusted Gauss-Kruger family",
            area_of_use: "Central and Eastern Europe",
            unit: "metre",
        });
    }
    if code == 2550 {
        return Some(EpsgInfo {
            code,
            name: "Samboja / UTM zone 50S",
            area_of_use: "Indonesia",
            unit: "metre",
        });
    }
    if code == 2600 {
        return Some(EpsgInfo {
            code,
            name: "LKS 1994 / Lithuania TM",
            area_of_use: "Lithuania",
            unit: "metre",
        });
    }
    if code == 2736 || code == 2737 {
        return Some(EpsgInfo {
            code,
            name: "Tete / UTM",
            area_of_use: "Mozambique",
            unit: "metre",
        });
    }
    if (3580..=3751).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83/NSRS2007 StatePlane and related systems",
            area_of_use: "North America and territories",
            unit: "metre/foot",
        });
    }
    if (26901..=26923).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83 / UTM (northern hemisphere)",
            area_of_use: "North America",
            unit: "metre",
        });
    }
    if (6328..=6348).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(2011) / UTM (northern hemisphere)",
            area_of_use: "North America",
            unit: "metre",
        });
    }
    if matches!(code, 2955..=2962 | 3154..=3160 | 3761 | 9709 | 9713) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS) / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22207..=22222).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v2 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22307..=22322).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v3 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22407..=22422).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v4 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22607..=22622).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v6 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22707..=22722).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v7 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (22807..=22822).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD83(CSRS)v8 / UTM (northern hemisphere)",
            area_of_use: "Canada",
            unit: "metre",
        });
    }
    if (26701..=26722).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "NAD27 / UTM (northern hemisphere)",
            area_of_use: "North America",
            unit: "metre",
        });
    }
    if (25801..=25860).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "ETRS89 / UTM (northern hemisphere)",
            area_of_use: "Europe",
            unit: "metre",
        });
    }
    if (23001..=23060).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "ED50 / UTM (northern hemisphere)",
            area_of_use: "Europe",
            unit: "metre",
        });
    }
    if (31965..=31976).contains(&code) || (6210..=6211).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "SIRGAS 2000 / UTM (northern hemisphere)",
            area_of_use: "South/Central America",
            unit: "metre",
        });
    }
    if (31977..=31985).contains(&code) || code == 5396 {
        return Some(EpsgInfo {
            code,
            name: "SIRGAS 2000 / UTM (southern hemisphere)",
            area_of_use: "South America",
            unit: "metre",
        });
    }
    if code == 5463 || (29168..=29172).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "SAD69 / UTM (northern hemisphere)",
            area_of_use: "South America",
            unit: "metre",
        });
    }
    if (29187..=29195).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "SAD69 / UTM (southern hemisphere)",
            area_of_use: "South America",
            unit: "metre",
        });
    }
    if (24817..=24821).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "PSAD56 / UTM (northern hemisphere)",
            area_of_use: "South America",
            unit: "metre",
        });
    }
    if (24877..=24882).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "PSAD56 / UTM (southern hemisphere)",
            area_of_use: "South America",
            unit: "metre",
        });
    }
    if (7849..=7856).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "GDA2020 / MGA",
            area_of_use: "Australia",
            unit: "metre",
        });
    }
    if (2334..=2390).contains(&code) {
        return Some(EpsgInfo {
            code,
            name: "Xian 1980 / Gauss-Kruger",
            area_of_use: "China",
            unit: "metre",
        });
    }
    if legacy_parity_wkt(code).is_some() {
        let unit = if (4120..=4185).contains(&code) { "degree" } else { "mixed" };
        return Some(EpsgInfo {
            code,
            name: "Legacy workflows parity CRS",
            area_of_use: "Various",
            unit,
        });
    }
    if let Some(info) = generated_epsg_info(code) {
        return Some(info);
    }
    None
}

fn build_crs(code: u32) -> Result<Crs> {
    if let Some(raw_wkt) = legacy_parity_wkt(code) {
        let wkt = raw_wkt
            .replace("\\\"", "\"")
            .replace("Gauss_Kruger", "Transverse_Mercator")
            .replace("Double_Stereographic", "Stereographic");
        return crate::wkt::parse_crs_from_wkt(&wkt);
    }
    if let Some(raw_wkt) = generated_epsg_wkt(code) {
        let wkt = raw_wkt
            .replace("\\\"", "\"")
            .replace("Gauss_Kruger", "Transverse_Mercator")
            .replace("Double_Stereographic", "Stereographic");
        return crate::wkt::parse_crs_from_wkt(&wkt);
    }

    // ── NAD83(NSRS2007) state-plane and related family block 3580–3751 ───
    if (3580..=3751).contains(&code) {
        return build_epsg_3580_3751(code);
    }


    // ── Xian 1980 GK families 2334–2390 ─────────────────────────────────
    if (2334..=2390).contains(&code) {
        return xian_1980_gk_crs(code);
    }

    // ── Pulkovo 1942 Gauss-Kruger CM (6-degree style) 2494–2522 ─────────
    if (2494..=2522).contains(&code) {
        let idx = (code - 2494) as i32;
        let lon0 = wrap_longitude_180(21.0 + 6.0 * f64::from(idx));
        return pulkovo_gk_cm(code, "Pulkovo 1942", lon0);
    }

    // ── Pulkovo 1995 Gauss-Kruger CM (6-degree style) 2463–2491 ─────────
    if (2463..=2491).contains(&code) {
        let idx = (code - 2463) as i32;
        let lon0 = wrap_longitude_180(21.0 + 6.0 * f64::from(idx));
        return pulkovo_gk_cm(code, "Pulkovo 1995", lon0);
    }

    // ── Pulkovo 1942 3-degree GK zone families 2523–2581 (except 2550) ──
    if (2523..=2581).contains(&code) && code != 2550 {
        let zone = if code < 2550 { code - 2516 } else { code - 2517 };
        return pulkovo_gk_zone(code, "Pulkovo 1942", zone);
    }

    // ── Pulkovo 1942 3-degree GK CM families 2582–2640 (except 2600) ─────
    if (2582..=2640).contains(&code) && code != 2600 {
        let mut idx = (code - 2582) as i32;
        if code > 2600 {
            idx -= 1;
        }
        let lon0 = wrap_longitude_180(21.0 + 3.0 * f64::from(idx));
        return pulkovo_gk_cm(code, "Pulkovo 1942", lon0);
    }

    // ── Pulkovo 1995 3-degree GK zone families 2641–2698 ─────────────────
    if (2641..=2698).contains(&code) {
        let zone = code - 2634; // 7..64
        return pulkovo_gk_zone(code, "Pulkovo 1995", zone);
    }

    // ── Pulkovo 1995 3-degree GK CM families 2699–2758 (except 2736,2737) ─
    if (2699..=2758).contains(&code) && code != 2736 && code != 2737 {
        let mut idx = (code - 2699) as i32;
        if code > 2737 {
            idx -= 2;
        }
        let lon0 = wrap_longitude_180(21.0 + 3.0 * f64::from(idx));
        return pulkovo_gk_cm(code, "Pulkovo 1995", lon0);
    }

    // ── Pulkovo 1995 / 1942 Gauss-Kruger zone families (6-degree) ───────
    if (20004..=20032).contains(&code) {
        let zone = code - 20000; // 4..32
        return pulkovo_gk_6deg_zone(code, "Pulkovo 1995", zone);
    }

    if (28404..=28432).contains(&code) {
        let zone = code - 28400; // 4..32
        return pulkovo_gk_6deg_zone(code, "Pulkovo 1942", zone);
    }

    // ── Outliers inside 2494–2758 family block ───────────────────────────
    if code == 2550 {
        return Ok(Crs {
            name: "Samboja / UTM zone 50S (EPSG:2550)".into(),
            datum: Datum {
                name: "Samboja",
                ellipsoid: Ellipsoid::BESSEL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(50, true)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        });
    }

    if code == 2600 {
        return Ok(Crs {
            name: "LKS 1994 / Lithuania TM (EPSG:2600)".into(),
            datum: Datum {
                name: "Lithuania 1994",
                ellipsoid: Ellipsoid::GRS80,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(24.0)
                    .with_lat0(0.0)
                    .with_scale(0.9998)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        });
    }

    if code == 2736 || code == 2737 {
        let zone: u8 = if code == 2736 { 36 } else { 37 };
        return Ok(Crs {
            name: format!("Tete / UTM zone {}S (EPSG:{code})", zone),
            datum: Datum {
                name: "Tete",
                ellipsoid: Ellipsoid::CLARKE1866,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, true)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        });
    }

    // ── WGS72 UTM northern zones 32201–32260 ────────────────────────────
    if (32201..=32260).contains(&code) {
        let zone = (code - 32200) as u8;
        return wgs72_utm_crs(code, zone, false);
    }

    // ── WGS72 UTM southern zones 32301–32360 ────────────────────────────
    if (32301..=32360).contains(&code) {
        let zone = (code - 32300) as u8;
        return wgs72_utm_crs(code, zone, true);
    }

    // ── WGS72BE UTM northern zones 32401–32460 ──────────────────────────
    if (32401..=32460).contains(&code) {
        let zone = (code - 32400) as u8;
        return wgs72be_utm_crs(code, zone, false);
    }

    // ── WGS72BE UTM southern zones 32501–32560 ──────────────────────────
    if (32501..=32560).contains(&code) {
        let zone = (code - 32500) as u8;
        return wgs72be_utm_crs(code, zone, true);
    }

    // ── WGS84 UTM northern zones 32601–32660 ─────────────────────────────
    if (32601..=32660).contains(&code) {
        let zone = (code - 32600) as u8;
        return Ok(Crs {
            name: format!("WGS 84 / UTM zone {}N (EPSG:{code})", zone),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, false),
            )?,
        });
    }

    // ── WGS84 UTM southern zones 32701–32760 ─────────────────────────────
    if (32701..=32760).contains(&code) {
        let zone = (code - 32700) as u8;
        return Ok(Crs {
            name: format!("WGS 84 / UTM zone {}S (EPSG:{code})", zone),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, true),
            )?,
        });
    }

    // ── NAD83 UTM northern zones 26901–26923 ─────────────────────────────
    if (26901..=26923).contains(&code) {
        let zone = (code - 26900) as u8;
        return Ok(Crs {
            name: format!("NAD83 / UTM zone {}N (EPSG:{code})", zone),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        });
    }

    // ── NAD83(2011) UTM northern zones 6328–6348 ───────────────────────
    if (6328..=6348).contains(&code) {
        let zone = match code {
            6328 => 59,
            6329 => 60,
            _ => (code - 6329) as u8,
        };
        return nad83_2011_utm_crs(code, zone);
    }

    // ── NAD27 UTM northern zones 26701–26722 ─────────────────────────────
    if (26701..=26722).contains(&code) {
        let zone = (code - 26700) as u8;
        return Ok(Crs {
            name: format!("NAD27 / UTM zone {}N (EPSG:{code})", zone),
            datum: Datum::NAD27,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        });
    }

    // ── ETRS89 UTM northern zones 25801–25860 ────────────────────────────
    if (25801..=25860).contains(&code) {
        let zone = (code - 25800) as u8;
        return Ok(Crs {
            name: format!("ETRS89 / UTM zone {}N (EPSG:{code})", zone),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        });
    }

    // ── ED50 UTM northern zones 23001–23060 ──────────────────────────────
    if (23001..=23060).contains(&code) {
        let zone = (code - 23000) as u8;
        return Ok(Crs {
            name: format!("ED50 / UTM zone {}N (EPSG:{code})", zone),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(zone, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        });
    }

    // ── SIRGAS 2000 UTM (active EPSG set) ────────────────────────────────
    if (31965..=31976).contains(&code) {
        let zone = (code - 31954) as u8; // 11..22N
        return sirgas2000_utm_crs(code, zone, false);
    }
    if (31977..=31985).contains(&code) {
        let zone = (code - 31960) as u8; // 17..25S
        return sirgas2000_utm_crs(code, zone, true);
    }
    if code == 6210 {
        return sirgas2000_utm_crs(code, 23, false);
    }
    if code == 6211 {
        return sirgas2000_utm_crs(code, 24, false);
    }
    if code == 5396 {
        return sirgas2000_utm_crs(code, 26, true);
    }

    // ── SAD69 UTM (active EPSG set only) ─────────────────────────────────
    if code == 5463 {
        return sad69_utm_crs(code, 17, false);
    }
    if (29168..=29172).contains(&code) {
        let zone = (code - 29150) as u8; // 18..22N
        return sad69_utm_crs(code, zone, false);
    }
    if (29187..=29195).contains(&code) {
        let zone = (code - 29170) as u8; // 17..25S
        return sad69_utm_crs(code, zone, true);
    }

    // ── PSAD56 UTM ────────────────────────────────────────────────────────
    if (24817..=24821).contains(&code) {
        let zone = (code - 24800) as u8; // 17..21N
        return psad56_utm_crs(code, zone, false);
    }
    if (24877..=24882).contains(&code) {
        let zone = (code - 24860) as u8; // 17..22S
        return psad56_utm_crs(code, zone, true);
    }

    // ── GDA2020 MGA zones 7849–7856 ──────────────────────────────────────
    if (7846..=7848).contains(&code) {
        let zone = (code - 7800) as u8; // 46..48
        return gda2020_mga_crs(code, zone);
    }

    // ── GDA2020 MGA zones 7849–7856 ──────────────────────────────────────
    if (7849..=7856).contains(&code) {
        let zone = (code - 7800) as u8; // 49..56
        return gda2020_mga_crs(code, zone);
    }

    // ── Named entries ─────────────────────────────────────────────────────
    match code {
        // ── Geographic 2D (Plate Carrée with unit-scale) ─────────────────
        4326 => geographic_crs("WGS 84 (EPSG:4326)", Datum::WGS84),
        4269 => geographic_crs("NAD83 (EPSG:4269)", Datum::NAD83),
        4267 => geographic_crs("NAD27 (EPSG:4267)", Datum::NAD27),
        4258 => geographic_crs("ETRS89 (EPSG:4258)", Datum::ETRS89),
        4230 => geographic_crs("ED50 (EPSG:4230)", Datum::ED50),
        4490 => geographic_crs("CGCS2000 (EPSG:4490)", Datum::CGCS2000),
        4674 => geographic_crs("SIRGAS 2000 (EPSG:4674)", Datum::SIRGAS2000),
        7843 => geographic_crs("GDA2020 (EPSG:7843)", Datum::GDA2020),
        7844 => geographic_crs("GDA2020 (EPSG:7844)", Datum::GDA2020),
        7842 => Ok(Crs {
            name: "GDA2020 geocentric (EPSG:7842)".into(),
            datum: Datum::GDA2020,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Geocentric)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        7841 => Ok(Crs {
            name: "GDA2020 height (EPSG:7841)".into(),
            datum: Datum::GDA2020,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        3855 => Ok(Crs {
            name: "EGM2008 height (EPSG:3855)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),
        5702 => Ok(Crs {
            name: "NGVD 29 height (EPSG:5702)".into(),
            datum: Datum::NAD27,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),
        5701 => Ok(Crs {
            name: "ODN height (EPSG:5701)".into(),
            datum: Datum::OSGB36,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::AIRY1830),
            )?,
        }),
        5703 => Ok(Crs {
            name: "NAVD88 height (EPSG:5703)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        5714 => Ok(Crs {
            name: "MSL height (EPSG:5714)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),
        5715 => Ok(Crs {
            name: "MSL depth (EPSG:5715)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),
        5773 => Ok(Crs {
            name: "EGM96 height (EPSG:5773)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),
        8228 => Ok(Crs {
            name: "NAVD88 height (EPSG:8228)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Vertical)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        4617 => geographic_crs("NAD83(CSRS) (EPSG:4617)", Datum::NAD83_CSRS),
        4954 => geographic_crs("NAD83(CSRS) (EPSG:4954)", Datum::NAD83_CSRS),
        4955 => geographic_crs("NAD83(CSRS) (EPSG:4955)", Datum::NAD83_CSRS),
        8230 => geographic_crs("NAD83(CSRS96) (EPSG:8230)", Datum::NAD83_CSRS),
        8231 => geographic_crs("NAD83(CSRS96) (EPSG:8231)", Datum::NAD83_CSRS),
        8232 => geographic_crs("NAD83(CSRS96) (EPSG:8232)", Datum::NAD83_CSRS),
        8233 => geographic_crs("NAD83(CSRS)v2 (EPSG:8233)", Datum::NAD83_CSRS),
        8235 => geographic_crs("NAD83(CSRS)v2 (EPSG:8235)", Datum::NAD83_CSRS),
        8237 => geographic_crs("NAD83(CSRS)v2 (EPSG:8237)", Datum::NAD83_CSRS),
        8238 => geographic_crs("NAD83(CSRS)v3 (EPSG:8238)", Datum::NAD83_CSRS),
        8239 => geographic_crs("NAD83(CSRS)v3 (EPSG:8239)", Datum::NAD83_CSRS),
        8240 => geographic_crs("NAD83(CSRS)v3 (EPSG:8240)", Datum::NAD83_CSRS),
        8242 => geographic_crs("NAD83(CSRS)v4 (EPSG:8242)", Datum::NAD83_CSRS),
        8244 => geographic_crs("NAD83(CSRS)v4 (EPSG:8244)", Datum::NAD83_CSRS),
        8246 => geographic_crs("NAD83(CSRS)v4 (EPSG:8246)", Datum::NAD83_CSRS),
        8247 => geographic_crs("NAD83(CSRS)v5 (EPSG:8247)", Datum::NAD83_CSRS),
        8248 => geographic_crs("NAD83(CSRS)v5 (EPSG:8248)", Datum::NAD83_CSRS),
        8249 => geographic_crs("NAD83(CSRS)v5 (EPSG:8249)", Datum::NAD83_CSRS),
        8250 => geographic_crs("NAD83(CSRS)v6 (EPSG:8250)", Datum::NAD83_CSRS),
        8251 => geographic_crs("NAD83(CSRS)v6 (EPSG:8251)", Datum::NAD83_CSRS),
        8252 => geographic_crs("NAD83(CSRS)v6 (EPSG:8252)", Datum::NAD83_CSRS),
        8253 => geographic_crs("NAD83(CSRS)v7 (EPSG:8253)", Datum::NAD83_CSRS),
        8254 => geographic_crs("NAD83(CSRS)v7 (EPSG:8254)", Datum::NAD83_CSRS),
        8255 => geographic_crs("NAD83(CSRS)v7 (EPSG:8255)", Datum::NAD83_CSRS),
        10413 => geographic_crs("NAD83(CSRS)v8 (EPSG:10413)", Datum::NAD83_CSRS),
        10414 => geographic_crs("NAD83(CSRS)v8 (EPSG:10414)", Datum::NAD83_CSRS),
        4601 => geographic_crs("Antigua 1943 (EPSG:4601)", Datum::ANTIGUA_1943),
        4602 => geographic_crs("Dominica 1945 (EPSG:4602)", Datum::DOMINICA_1945),
        4603 => geographic_crs("Grenada 1953 (EPSG:4603)", Datum::GRENADA_1953),
        4604 => geographic_crs("Montserrat 1958 (EPSG:4604)", Datum::MONTSERRAT_1958),
        4605 => geographic_crs("St. Kitts 1955 (EPSG:4605)", Datum::ST_KITTS_1955),
        4610 => geographic_crs("Xian 1980 (EPSG:4610)", Datum::XIAN_1980),
        4612 => geographic_crs("JGD2000 (EPSG:4612)", Datum::JGD2000),

        // ── Legacy workflows parity block (4001–4063 selected) ─────────
        4001 => custom_geographic_crs(4001, "GCS_Airy_1830", "D_Airy_1830", Ellipsoid::AIRY1830),
        4002 => custom_geographic_crs(4002, "GCS_Airy_Modified", "D_Airy_Modified", Ellipsoid::AIRY1830_MOD),
        4003 => custom_geographic_crs(4003, "GCS_Australian", "D_Australian", Ellipsoid::from_a_inv_f("Australian", 6_378_160.0, 298.25)),
        4004 => custom_geographic_crs(4004, "GCS_Bessel_1841", "D_Bessel_1841", Ellipsoid::BESSEL),
        4005 => custom_geographic_crs(4005, "GCS_Bessel_Modified", "D_Bessel_Modified", Ellipsoid::from_a_inv_f("Bessel Modified", 6_377_492.018, 299.152_812_8)),
        4006 => custom_geographic_crs(4006, "GCS_Bessel_Namibia", "D_Bessel_Namibia", Ellipsoid::from_a_inv_f("Bessel Namibia", 6_377_483.865_280_418, 299.152_812_8)),
        4007 => custom_geographic_crs(4007, "GCS_Clarke_1858", "D_Clarke_1858", Ellipsoid::from_a_inv_f("Clarke 1858", 6_378_293.645_208_759, 294.260_676_369)),
        4008 => custom_geographic_crs(4008, "GCS_Clarke_1866", "D_Clarke_1866", Ellipsoid::CLARKE1866),
        4009 => custom_geographic_crs(4009, "GCS_Clarke_1866_Michigan", "D_Clarke_1866_Michigan", Ellipsoid::from_a_inv_f("Clarke 1866 Michigan", 6_378_450.047, 294.978_684_677)),
        4010 => custom_geographic_crs(4010, "GCS_Clarke_1880_Benoit", "D_Clarke_1880_Benoit", Ellipsoid::from_a_inv_f("Clarke 1880 Benoit", 6_378_300.789, 293.466_315_538_980_2)),
        4011 => custom_geographic_crs(4011, "GCS_Clarke_1880_IGN", "D_Clarke_1880_IGN", Ellipsoid::from_a_inv_f("Clarke 1880 IGN", 6_378_249.2, 293.466_021_293_626_5)),
        4012 => custom_geographic_crs(4012, "GCS_Clarke_1880_RGS", "D_Clarke_1880_RGS", Ellipsoid::from_a_inv_f("Clarke 1880 RGS", 6_378_249.145, 293.465)),
        4013 => custom_geographic_crs(4013, "GCS_Clarke_1880_Arc", "D_Clarke_1880_Arc", Ellipsoid::from_a_inv_f("Clarke 1880 Arc", 6_378_249.145, 293.466_307_656)),
        4014 => custom_geographic_crs(4014, "GCS_Clarke_1880_SGA", "D_Clarke_1880_SGA", Ellipsoid::from_a_inv_f("Clarke 1880 SGA", 6_378_249.2, 293.465_98)),
        4015 => custom_geographic_crs(4015, "GCS_Everest_Adj_1937", "D_Everest_Adj_1937", Ellipsoid::from_a_inv_f("Everest Adjustment 1937", 6_377_276.345, 300.8017)),
        4016 => custom_geographic_crs(4016, "GCS_Everest_def_1967", "D_Everest_Def_1967", Ellipsoid::from_a_inv_f("Everest Definition 1967", 6_377_298.556, 300.8017)),
        4018 => custom_geographic_crs(4018, "GCS_Everest_Modified", "D_Everest_Modified", Ellipsoid::from_a_inv_f("Everest 1830 Modified", 6_377_304.063, 300.8017)),
        4019 => custom_geographic_crs(4019, "GCS_GRS_1980", "D_GRS_1980", Ellipsoid::GRS80),
        4020 => custom_geographic_crs(4020, "GCS_Helmert_1906", "D_Helmert_1906", Ellipsoid::HELMERT1906),
        4021 => custom_geographic_crs(4021, "GCS_Indonesian", "D_Indonesian", Ellipsoid::from_a_inv_f("Indonesian", 6_378_160.0, 298.247)),
        4022 => custom_geographic_crs(4022, "GCS_International_1924", "D_International_1924", Ellipsoid::INTERNATIONAL),
        4023 => custom_geographic_crs(4023, "GCS_MOLDREF99", "D_MOLDREF99", Ellipsoid::GRS80),
        4024 => custom_geographic_crs(4024, "GCS_Krasovsky_1940", "D_Krasovsky_1940", Ellipsoid::KRASSOWSKY1940),
        4025 => custom_geographic_crs(4025, "GCS_NWL_9D", "D_NWL_9D", Ellipsoid::from_a_inv_f("NWL 9D", 6_378_145.0, 298.25)),
        4026 => custom_tm_crs(4026, "MOLDREF99_Moldova_TM", "D_MOLDREF99", Ellipsoid::GRS80, 28.4, 0.0, 0.99994, 200_000.0, -5_000_000.0),
        4027 => custom_geographic_crs(4027, "GCS_Plessis_1817", "D_Plessis_1817", Ellipsoid::from_a_inv_f("Plessis 1817", 6_376_523.0, 308.64)),
        4028 => custom_geographic_crs(4028, "GCS_Struve_1860", "D_Struve_1860", Ellipsoid::from_a_inv_f("Struve 1860", 6_378_298.3, 294.73)),
        4029 => custom_geographic_crs(4029, "GCS_War_Office", "D_War_Office", Ellipsoid::from_a_inv_f("War Office", 6_378_300.0, 296.0)),
        4031 => custom_geographic_crs(4031, "GCS_GEM_10C", "D_GEM_10C", Ellipsoid::from_a_inv_f("GEM 10C", 6_378_137.0, 298.257_223_563)),
        4032 => custom_geographic_crs(4032, "GCS_OSU_86F", "D_OSU_86F", Ellipsoid::from_a_inv_f("OSU 86F", 6_378_136.2, 298.257_223_563)),
        4033 => custom_geographic_crs(4033, "GCS_OSU_91A", "D_OSU_91A", Ellipsoid::from_a_inv_f("OSU 91A", 6_378_136.3, 298.257_223_563)),
        4034 => custom_geographic_crs(4034, "GCS_Clarke_1880", "D_Clarke_1880", Ellipsoid::from_a_inv_f("Clarke 1880", 6_378_249.144_808_011, 293.466_307_655_625_3)),
        4035 => custom_geographic_crs(4035, "GCS_Sphere", "D_Sphere", Ellipsoid::sphere("Sphere", 6_371_000.0)),
        4036 => custom_geographic_crs(4036, "GCS_GRS_1967", "D_GRS_1967", Ellipsoid::from_a_inv_f("GRS 1967", 6_378_160.0, 298.247_167_427)),
        4037 => custom_tm_crs(4037, "WGS_1984_TMzn35N", "D_WGS_1984", Ellipsoid::WGS84, 27.0, 0.0, 0.9996, 500_000.0, 0.0),
        4038 => custom_tm_crs(4038, "WGS_1984_TMzn36N", "D_WGS_1984", Ellipsoid::WGS84, 33.0, 0.0, 0.9996, 500_000.0, 0.0),
        4044 => custom_geographic_crs(4044, "GCS_Everest_def_1962", "D_Everest_Def_1962", Ellipsoid::from_a_inv_f("Everest Definition 1962", 6_377_301.243, 300.801_725_5)),
        4045 => custom_geographic_crs(4045, "GCS_Everest_def_1975", "D_Everest_Def_1975", Ellipsoid::from_a_inv_f("Everest Definition 1975", 6_377_299.151, 300.801_725_5)),
        4046 => custom_geographic_crs(4046, "GCS_RGRDC_2005", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80),
        4047 => custom_geographic_crs(4047, "GCS_Sphere_GRS_1980_Authalic", "D_Sphere_GRS_1980_Authalic", Ellipsoid::sphere("Sphere GRS 1980 Authalic", 6_371_007.0)),
        4048 => custom_tm_crs(4048, "RGRDC_2005_Congo_TM_Zone_12", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 12.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4049 => custom_tm_crs(4049, "RGRDC_2005_Congo_TM_Zone_14", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 14.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4050 => custom_tm_crs(4050, "RGRDC_2005_Congo_TM_Zone_16", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 16.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4051 => custom_tm_crs(4051, "RGRDC_2005_Congo_TM_Zone_18", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 18.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4052 => custom_geographic_crs(4052, "GCS_Sphere_Clarke_1866_Authalic", "D_Sphere_Clarke_1866_Authalic", Ellipsoid::sphere("Sphere Clarke 1866 Authalic", 6_370_997.0)),
        4053 => custom_geographic_crs(4053, "GCS_Sphere_International_1924_Authalic", "D_Sphere_International_1924_Authalic", Ellipsoid::sphere("Sphere International 1924 Authalic", 6_371_228.0)),
        4054 => custom_geographic_crs(4054, "GCS_Hughes_1980", "D_Hughes_1980", Ellipsoid::from_a_inv_f("Hughes 1980", 6_378_273.0, 298.279_411_123_064)),
        4055 => custom_geographic_crs(4055, "GCS_WGS_1984_Major_Auxiliary_Sphere", "D_WGS_1984_Major_Auxiliary_Sphere", Ellipsoid::sphere("WGS 1984 Major Auxiliary Sphere", 6_378_137.0)),
        4056 => custom_tm_crs(4056, "RGRDC_2005_Congo_TM_Zone_20", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 20.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4057 => custom_tm_crs(4057, "RGRDC_2005_Congo_TM_Zone_22", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 22.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4058 => custom_tm_crs(4058, "RGRDC_2005_Congo_TM_Zone_24", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 24.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4059 => custom_tm_crs(4059, "RGRDC_2005_Congo_TM_Zone_26", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 26.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4060 => custom_tm_crs(4060, "RGRDC_2005_Congo_TM_Zone_28", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 28.0, 0.0, 0.9999, 500_000.0, 10_000_000.0),
        4061 => custom_tm_crs(4061, "RGRDC_2005_UTM_Zone_33S", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 15.0, 0.0, 0.9996, 500_000.0, 10_000_000.0),
        4062 => custom_tm_crs(4062, "RGRDC_2005_UTM_Zone_34S", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 21.0, 0.0, 0.9996, 500_000.0, 10_000_000.0),
        4063 => custom_tm_crs(4063, "RGRDC_2005_UTM_Zone_35S", "D_Reseau_Geodesique_de_la_RDC_2005", Ellipsoid::GRS80, 27.0, 0.0, 0.9996, 500_000.0, 10_000_000.0),

        // ── Web / World Mercator ─────────────────────────────────────────
        3857 => Ok(Crs::web_mercator()),

        3395 => Ok(Crs {
            name: "WGS 84 / World Mercator (EPSG:3395)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Mercator),
            )?,
        }),

        4491..=4501 => {
            let zone = code - 4478; // 13..23
            let lon0 = 75.0 + 6.0 * f64::from(code - 4491);
            cgcs2000_gk_zone_crs(code, zone, lon0)
        }

        4502..=4512 => {
            let lon0 = 75.0 + 6.0 * f64::from(code - 4502);
            cgcs2000_gk_cm_crs(code, lon0)
        }

        4513..=4533 => {
            let zone = code - 4488; // 25..45
            let lon0 = 75.0 + 3.0 * f64::from(code - 4513);
            cgcs2000_gk_3deg_zone_crs(code, zone, lon0)
        }

        4534..=4537 => {
            let lon0 = 75.0 + 3.0 * f64::from(code - 4534);
            cgcs2000_gk_3deg_cm_crs(code, lon0)
        }

        4538..=4554 => {
            let lon0 = 87.0 + 3.0 * f64::from(code - 4538);
            cgcs2000_gk_3deg_cm_crs(code, lon0)
        }

        4568..=4578 => {
            let zone = code - 4555; // 13..23
            let lon0 = 75.0 + 6.0 * f64::from(code - 4568);
            new_beijing_gk_zone_crs(code, zone, lon0)
        }

        4579..=4589 => {
            let lon0 = 75.0 + 6.0 * f64::from(code - 4579);
            new_beijing_gk_cm_crs(code, lon0)
        }

        4652..=4656 => {
            let zone = code - 4627; // 25..29
            let lon0 = 75.0 + 3.0 * f64::from(code - 4652);
            new_beijing_gk_3deg_zone_crs(code, zone, lon0)
        }

        4766..=4781 => {
            let zone = code - 4736; // 30..45
            let lon0 = 90.0 + 3.0 * f64::from(code - 4766);
            new_beijing_gk_3deg_zone_crs(code, zone, lon0)
        }

        4782..=4790 => {
            let lon0 = 75.0 + 3.0 * f64::from(code - 4782);
            new_beijing_gk_3deg_cm_crs(code, lon0)
        }

        4791..=4800 => {
            let lon0 = 102.0 + 3.0 * f64::from(code - 4791);
            new_beijing_gk_3deg_cm_crs(code, lon0)
        }

        4812 => new_beijing_gk_3deg_cm_crs(code, 132.0),

        4822 => new_beijing_gk_3deg_cm_crs(code, 135.0),

        4855..=4867 => {
            let zone = code - 4850; // 5..17
            let lon0 = f64::from(zone) + 0.5;
            etrs89_nor_ntm_crs(code, zone, lon0)
        }

        5105..=5129 => {
            let zone = code - 5100; // 5..29
            let lon0 = f64::from(zone) + 0.5;
            etrs89_nor_ntm_crs(code, zone, lon0)
        }

        3400 => Ok(Crs {
            name: "NAD83 / Alberta 10-TM (Forest) (EPSG:3400)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(-115.0)
                    .with_scale(0.9992)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3401 => Ok(Crs {
            name: "NAD83 / Alberta 10-TM (Resource) (EPSG:3401)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(-115.0)
                    .with_scale(0.9992)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3402 => Ok(Crs {
            name: "NAD83(CSRS) / Alberta 10-TM (Forest) (EPSG:3402)".into(),
            datum: Datum::NAD83_CSRS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(-115.0)
                    .with_scale(0.9992)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3403 => Ok(Crs {
            name: "NAD83(CSRS) / Alberta 10-TM (Resource) (EPSG:3403)".into(),
            datum: Datum::NAD83_CSRS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(-115.0)
                    .with_scale(0.9992)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3405 => Ok(Crs {
            name: "VN-2000 / UTM zone 48N (EPSG:3405)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(48, false)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3406 => Ok(Crs {
            name: "VN-2000 / UTM zone 49N (EPSG:3406)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(49, false)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2163 => Ok(Crs {
            name: "US National Atlas Equal Area (EPSG:2163)".into(),
            datum: Datum::WGS84, // Approximate; spherical authalic model used
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(45.0)
                    .with_lon0(-100.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::sphere("Clarke 1866 Authalic Sphere", 6_370_997.0)),
            )?,
        }),

        3408 => Ok(Crs {
            name: "NSIDC EASE-Grid North (EPSG:3408)".into(),
            datum: Datum::WGS84, // Approximate; authalic sphere model used
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::sphere("NSIDC Authalic Sphere", 6_371_228.0)),
            )?,
        }),

        3409 => Ok(Crs {
            name: "NSIDC EASE-Grid South (EPSG:3409)".into(),
            datum: Datum::WGS84, // Approximate; authalic sphere model used
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(-90.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::sphere("NSIDC Authalic Sphere", 6_371_228.0)),
            )?,
        }),

        32662 => Ok(Crs {
            name: "WGS 84 / Plate Carree (EPSG:32662)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Equirectangular { lat_ts: 0.0 }),
            )?,
        }),

        32661 => Ok(Crs {
            name: "WGS 84 / UPS North (N,E) (EPSG:32661)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(90.0)
                    .with_lon0(0.0)
                    .with_scale(0.994)
                    .with_false_easting(2_000_000.0)
                    .with_false_northing(2_000_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        32761 => Ok(Crs {
            name: "WGS 84 / UPS South (N,E) (EPSG:32761)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(-90.0)
                    .with_lon0(0.0)
                    .with_scale(0.994)
                    .with_false_easting(2_000_000.0)
                    .with_false_northing(2_000_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        4087 => Ok(Crs {
            name: "WGS 84 / World Equidistant Cylindrical (EPSG:4087)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Equirectangular { lat_ts: 0.0 })
                    .with_lat0(0.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        // ── NAD83(CSRS) UTM (Canada; active v1 set) ────────────────────
        2955 => csrs_utm_crs(11, code),
        2956 => csrs_utm_crs(12, code),
        2957 => csrs_utm_crs(13, code),
        2958 => csrs_utm_crs(17, code),
        2959 => csrs_utm_crs(18, code),
        2960 => csrs_utm_crs(19, code),
        2961 => csrs_utm_crs(20, code),
        2962 => csrs_utm_crs(21, code),
        3154 => csrs_utm_crs(7, code),
        3155 => csrs_utm_crs(8, code),
        3156 => csrs_utm_crs(9, code),
        3157 => csrs_utm_crs(10, code),
        3158 => csrs_utm_crs(14, code),
        3159 => csrs_utm_crs(15, code),
        3160 => csrs_utm_crs(16, code),
        3761 => csrs_utm_crs(22, code),
        9709 => csrs_utm_crs(23, code),
        9713 => csrs_utm_crs(24, code),
        // NAD83(CSRS) realization families (v2-v8)
        22207..=22222 => csrs_utm_crs_variant((code - 22200) as u8, code, "v2"),
        22307..=22322 => csrs_utm_crs_variant((code - 22300) as u8, code, "v3"),
        22407..=22422 => csrs_utm_crs_variant((code - 22400) as u8, code, "v4"),
        22607..=22622 => csrs_utm_crs_variant((code - 22600) as u8, code, "v6"),
        22707..=22722 => csrs_utm_crs_variant((code - 22700) as u8, code, "v7"),
        22807..=22822 => csrs_utm_crs_variant((code - 22800) as u8, code, "v8"),

        // ── ETRS89 pan-European ──────────────────────────────────────────
        3034 => Ok(Crs {
            name: "ETRS89 / LCC Europe (EPSG:3034)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 35.0,
                    lat2: Some(65.0),
                })
                .with_lat0(52.0)
                .with_lon0(10.0)
                .with_false_easting(4_000_000.0)
                .with_false_northing(2_800_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3035 => Ok(Crs {
            name: "ETRS89 / LAEA Europe (EPSG:3035)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(52.0)
                    .with_lon0(10.0)
                    .with_false_easting(4_321_000.0)
                    .with_false_northing(3_210_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── Polar stereographic ───────────────────────────────────────────
        3031 => Ok(Crs {
            name: "WGS 84 / Antarctic Polar Stereographic (EPSG:3031)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(-90.0)
                    .with_lon0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3032 => Ok(Crs {
            name: "WGS 84 / Australian Antarctic Polar Stereographic (EPSG:3032)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(-90.0)
                    .with_lon0(70.0)
                    .with_scale(polar_stereographic_variant_b_scale(-71.0, &Ellipsoid::WGS84))
                    .with_false_easting(6_000_000.0)
                    .with_false_northing(6_000_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3413 => Ok(Crs {
            name: "WGS 84 / NSIDC Sea Ice Polar Stereographic North (EPSG:3413)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(90.0)
                    .with_lon0(-45.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3976 => Ok(Crs {
            name: "WGS 84 / NSIDC Sea Ice Polar Stereographic South (EPSG:3976)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(-90.0)
                    .with_lon0(0.0)
                    .with_scale(polar_stereographic_variant_b_scale(-70.0, &Ellipsoid::WGS84))
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3995 => Ok(Crs {
            name: "WGS 84 / Arctic Polar Stereographic (EPSG:3995)".into(),
            datum: Datum::SOUTH_EAST_ISLAND_1943,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(90.0)
                    .with_lon0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3996 => Ok(Crs {
            name: "WGS 84 / IBCAO Polar Stereographic (EPSG:3996)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(90.0)
                    .with_lon0(0.0)
                    .with_scale(polar_stereographic_variant_b_scale(75.0, &Ellipsoid::WGS84))
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3575 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Europe (EPSG:3575)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(10.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3576 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Russia (EPSG:3576)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(90.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3571 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Bering Sea (EPSG:3571)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(180.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3572 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Alaska (EPSG:3572)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(-150.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3573 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Canada (EPSG:3573)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(-100.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3574 => Ok(Crs {
            name: "WGS 84 / North Pole LAEA Atlantic (EPSG:3574)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(-40.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        6931 => Ok(Crs {
            name: "WGS 84 / NSIDC EASE-Grid 2.0 North (EPSG:6931)".into(),
            datum: Datum::SVY21,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(90.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        6932 => Ok(Crs {
            name: "WGS 84 / NSIDC EASE-Grid 2.0 South (EPSG:6932)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                    .with_lat0(-90.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        6933 => Ok(Crs {
            name: "WGS 84 / NSIDC EASE-Grid 2.0 Global (EPSG:6933)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::CylindricalEqualArea { lat_ts: 30.0 })
                    .with_lat0(0.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3410 => Ok(Crs {
            name: "NSIDC EASE-Grid Global (EPSG:3410)".into(),
            datum: Datum::WGS84, // Approximate; authalic sphere model used
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::CylindricalEqualArea { lat_ts: 30.0 })
                    .with_lat0(0.0)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::sphere("NSIDC Authalic Sphere", 6_371_228.0)),
            )?,
        }),

        8857 => Ok(Crs {
            name: "WGS 84 / Equal Earth Greenwich (EPSG:8857)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::EqualEarth)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3832 => Ok(Crs {
            name: "WGS 84 / PDC Mercator (EPSG:3832)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Mercator)
                    .with_lat0(0.0)
                    .with_lon0(150.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3833 => Ok(Crs {
            name: "Pulkovo 1942(58) / Gauss-Kruger zone 2 (EPSG:3833)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(9.0)
                    .with_scale(1.0)
                    .with_false_easting(2_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3834 => Ok(Crs {
            name: "Pulkovo 1942(83) / Gauss-Kruger zone 2 (EPSG:3834)".into(),
            datum: Datum::PULKOVO1942_83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(9.0)
                    .with_scale(1.0)
                    .with_false_easting(2_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3835 => Ok(Crs {
            name: "Pulkovo 1942(83) / Gauss-Kruger zone 3 (EPSG:3835)".into(),
            datum: Datum::PULKOVO1942_83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(15.0)
                    .with_scale(1.0)
                    .with_false_easting(3_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3836 => Ok(Crs {
            name: "Pulkovo 1942(83) / Gauss-Kruger zone 4 (EPSG:3836)".into(),
            datum: Datum::PULKOVO1942_83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(21.0)
                    .with_scale(1.0)
                    .with_false_easting(4_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3837 => Ok(Crs {
            name: "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 3 (EPSG:3837)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(9.0)
                    .with_scale(1.0)
                    .with_false_easting(3_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3838 => Ok(Crs {
            name: "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 4 (EPSG:3838)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(12.0)
                    .with_scale(1.0)
                    .with_false_easting(4_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3839 => Ok(Crs {
            name: "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 9 (EPSG:3839)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(27.0)
                    .with_scale(1.0)
                    .with_false_easting(9_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3840 => Ok(Crs {
            name: "Pulkovo 1942(58) / 3-degree Gauss-Kruger zone 10 (EPSG:3840)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(30.0)
                    .with_scale(1.0)
                    .with_false_easting(10_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3841 => Ok(Crs {
            name: "Pulkovo 1942(83) / 3-degree Gauss-Kruger zone 6 (EPSG:3841)".into(),
            datum: Datum::PULKOVO1942_83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(18.0)
                    .with_scale(1.0)
                    .with_false_easting(6_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        3845 => Ok(Crs {
            name: "SWEREF99 / RT90 7.5 gon V emulation (EPSG:3845)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(11.30625)
                    .with_scale(1.000_006)
                    .with_false_easting(1_500_025.141)
                    .with_false_northing(-667.282)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3846 => Ok(Crs {
            name: "SWEREF99 / RT90 5 gon V emulation (EPSG:3846)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(13.556_266_666_666_7)
                    .with_scale(1.000_005_8)
                    .with_false_easting(1_500_044.695)
                    .with_false_northing(-667.13)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3847 => Ok(Crs {
            name: "SWEREF99 / RT90 2.5 gon V emulation (EPSG:3847)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(15.806_284_529_444_4)
                    .with_scale(1.000_005_610_24)
                    .with_false_easting(1_500_064.274)
                    .with_false_northing(-667.711)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3848 => Ok(Crs {
            name: "SWEREF99 / RT90 0 gon emulation (EPSG:3848)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(18.0563)
                    .with_scale(1.000_005_4)
                    .with_false_easting(1_500_083.521)
                    .with_false_northing(-668.844)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3849 => Ok(Crs {
            name: "SWEREF99 / RT90 2.5 gon O emulation (EPSG:3849)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(20.306_316_666_666_7)
                    .with_scale(1.000_005_2)
                    .with_false_easting(1_500_102.765)
                    .with_false_northing(-670.706)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3850 => Ok(Crs {
            name: "SWEREF99 / RT90 5 gon O emulation (EPSG:3850)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(22.556_333_333_333_3)
                    .with_scale(1.000_004_9)
                    .with_false_easting(1_500_121.846)
                    .with_false_northing(-672.557)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3986 => Ok(Crs {
            name: "Katanga 1955 / Katanga Gauss zone A (EPSG:3986)".into(),
            datum: Datum::KATANGA1955,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(-9.0)
                    .with_lon0(30.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        3987 => Ok(Crs {
            name: "Katanga 1955 / Katanga Gauss zone B (EPSG:3987)".into(),
            datum: Datum::KATANGA1955,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(-9.0)
                    .with_lon0(28.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        3988 => Ok(Crs {
            name: "Katanga 1955 / Katanga Gauss zone C (EPSG:3988)".into(),
            datum: Datum::KATANGA1955,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(-9.0)
                    .with_lon0(26.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        3989 => Ok(Crs {
            name: "Katanga 1955 / Katanga Gauss zone D (EPSG:3989)".into(),
            datum: Datum::KATANGA1955,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(-9.0)
                    .with_lon0(24.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        3997 => Ok(Crs {
            name: "WGS 84 / Dubai Local TM (EPSG:3997)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(55.333_333_333_333_3)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3994 => Ok(Crs {
            name: "WGS 84 / Mercator 41 (EPSG:3994)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Mercator)
                    .with_lat0(0.0)
                    .with_lon0(100.0)
                    .with_scale(mercator_variant_b_scale(-41.0, &Ellipsoid::WGS84))
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        3991 => {
            let us_survey_foot = 1200.0 / 3937.0;
            let clarke1866_a_ft = Ellipsoid::CLARKE1866.a / us_survey_foot;
            let clarke1866_inv_f = 1.0 / Ellipsoid::CLARKE1866.f;
            Ok(Crs {
                name: "Puerto Rico State Plane CS of 1927 (EPSG:3991)".into(),
                datum: Datum::PUERTO_RICO_1927,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::LambertConformalConic {
                        lat1: 18.433_333_333_333_3,
                        lat2: Some(18.033_333_333_333_3),
                    })
                    .with_lat0(17.833_333_333_333_3)
                    .with_lon0(-66.433_333_333_333_3)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1866 (ftUS)", clarke1866_a_ft, clarke1866_inv_f)),
                )?,
            })
        }

        3992 => {
            let us_survey_foot = 1200.0 / 3937.0;
            let clarke1866_a_ft = Ellipsoid::CLARKE1866.a / us_survey_foot;
            let clarke1866_inv_f = 1.0 / Ellipsoid::CLARKE1866.f;
            Ok(Crs {
                name: "Puerto Rico / St. Croix (EPSG:3992)".into(),
                datum: Datum::ST_CROIX,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::LambertConformalConic {
                        lat1: 18.433_333_333_333_3,
                        lat2: Some(18.033_333_333_333_3),
                    })
                    .with_lat0(17.833_333_333_333_3)
                    .with_lon0(-66.433_333_333_333_3)
                    .with_false_easting(500_000.0)
                    .with_false_northing(100_000.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1866 (ftUS)", clarke1866_a_ft, clarke1866_inv_f)),
                )?,
            })
        }

        54008 => Ok(Crs {
            name: "World Sinusoidal (ESRI:54008)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Sinusoidal)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        54009 => Ok(Crs {
            name: "World Mollweide (ESRI:54009)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Mollweide)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        54030 => Ok(Crs {
            name: "World Robinson (ESRI:54030)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Robinson)
                    .with_lon0(0.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        5070 => Ok(Crs {
            name: "NAD83 / Conus Albers (EPSG:5070)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 29.5,
                    lat2: 45.5,
                })
                .with_lat0(23.0)
                .with_lon0(-96.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3578 => Ok(Crs {
            name: "NAD83 / Yukon Albers (EPSG:3578)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 61.666_666_666_666_7,
                    lat2: 68.0,
                })
                .with_lat0(59.0)
                .with_lon0(-132.5)
                .with_false_easting(500_000.0)
                .with_false_northing(500_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3579 => Ok(Crs {
            name: "NAD83(CSRS) / Yukon Albers (EPSG:3579)".into(),
            datum: Datum::NAD83_CSRS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 61.666_666_666_666_7,
                    lat2: 68.0,
                })
                .with_lat0(59.0)
                .with_lon0(-132.5)
                .with_false_easting(500_000.0)
                .with_false_northing(500_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        
        3577 => Ok(Crs {
            name: "GDA94 / Australian Albers (EPSG:3577)".into(),
            datum: Datum::GDA94,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: -18.0,
                    lat2: -36.0,
                })
                .with_lat0(0.0)
                .with_lon0(132.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── UK / Ireland ─────────────────────────────────────────────────
        27700 => Ok(Crs {
            name: "OSGB 1936 / British National Grid (EPSG:27700)".into(),
            datum: Datum::OSGB36,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(49.0)
                    .with_lon0(-2.0)
                    .with_scale(0.9996012717)
                    .with_false_easting(400_000.0)
                    .with_false_northing(-100_000.0)
                    .with_ellipsoid(Ellipsoid::AIRY1830),
            )?,
        }),

        29900 => Ok(Crs {
            name: "TM65 / Irish National Grid (EPSG:29900)".into(),
            datum: Datum::TM65,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(53.5)
                    .with_lon0(-8.0)
                    .with_scale(1.000035)
                    .with_false_easting(200_000.0)
                    .with_false_northing(250_000.0)
                    .with_ellipsoid(Ellipsoid::AIRY1830_MOD),
            )?,
        }),

        29903 => Ok(Crs {
            name: "TM65 / Irish Grid (EPSG:29903)".into(),
            datum: Datum::TM65,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(53.5)
                    .with_lon0(-8.0)
                    .with_scale(1.000035)
                    .with_false_easting(200_000.0)
                    .with_false_northing(250_000.0)
                    .with_ellipsoid(Ellipsoid::AIRY1830_MOD),
            )?,
        }),

        2157 => Ok(Crs {
            name: "IRENET95 / Irish Transverse Mercator (EPSG:2157)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(53.5)
                    .with_lon0(-8.0)
                    .with_scale(0.99982)
                    .with_false_easting(600_000.0)
                    .with_false_northing(750_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2188 => Ok(Crs {
            name: "Azores Occidental 1939 / UTM zone 25N (EPSG:2188)".into(),
            datum: Datum {
                name: "Azores Occidental 1939",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(25, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2189 => Ok(Crs {
            name: "Azores Central 1948 / UTM zone 26N (EPSG:2189)".into(),
            datum: Datum {
                name: "Azores Central 1948",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(26, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2190 => Ok(Crs {
            name: "Azores Oriental 1940 / UTM zone 26N (EPSG:2190)".into(),
            datum: Datum {
                name: "Azores Oriental 1940",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(26, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2193 => Ok(Crs {
            name: "NZGD2000 / New Zealand Transverse Mercator 2000 (EPSG:2193)".into(),
            datum: Datum::NZGD2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(173.0)
                    .with_scale(0.9996)
                    .with_false_easting(1_600_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        2195 => Ok(Crs {
            name: "NAD83(HARN) / UTM zone 2S (EPSG:2195)".into(),
            datum: Datum::NAD83_HARN,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(2, true)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2196 => Ok(Crs {
            name: "ETRS89 / Kp2000 Jutland (EPSG:2196)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(9.5)
                    .with_scale(0.99995)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2197 => Ok(Crs {
            name: "ETRS89 / Kp2000 Zealand (EPSG:2197)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(12.0)
                    .with_scale(0.99995)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2198 => Ok(Crs {
            name: "ETRS89 / Kp2000 Bornholm (EPSG:2198)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(15.0)
                    .with_scale(1.0)
                    .with_false_easting(900_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2200 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / New Brunswick Stereographic (EPSG:2200)".into(),
                datum: datum.clone(),
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::Stereographic)
                        .with_lon0(-66.5)
                        .with_lat0(46.5)
                        .with_scale(0.999_912)
                        .with_false_easting(300_000.0)
                        .with_false_northing(800_000.0)
                        .with_ellipsoid(datum.ellipsoid.clone()),
                )?,
            })
        }

        2201 => Ok(Crs {
            name: "REGVEN / UTM zone 18N (EPSG:2201)".into(),
            datum: Datum {
                name: "REGVEN",
                ellipsoid: Ellipsoid::GRS80,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(18, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2202 => Ok(Crs {
            name: "REGVEN / UTM zone 19N (EPSG:2202)".into(),
            datum: Datum {
                name: "REGVEN",
                ellipsoid: Ellipsoid::GRS80,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(19, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2203 => Ok(Crs {
            name: "REGVEN / UTM zone 20N (EPSG:2203)".into(),
            datum: Datum {
                name: "REGVEN",
                ellipsoid: Ellipsoid::GRS80,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(20, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2204 => us_state_plane_lcc_ftus(
            2204,
            "NAD27 / Tennessee (FIPS 4100) ftUS",
            Datum::NAD27,
            -86.0,
            35.25,
            36.41666666666666,
            34.66666666666666,
            2_000_000.0,
            100_000.0,
        ),

        2205 => us_state_plane_lcc(
            2205,
            "NAD83 / Kentucky North (FIPS 1601)",
            Datum::NAD83,
            -84.25,
            37.96666666666667,
            38.96666666666667,
            37.5,
            500000.0,
            0.0,
        ),

        2206 => ed50_gk_3deg_zone(2206, 9),
        2207 => ed50_gk_3deg_zone(2207, 10),
        2208 => ed50_gk_3deg_zone(2208, 11),
        2209 => ed50_gk_3deg_zone(2209, 12),
        2210 => ed50_gk_3deg_zone(2210, 13),
        2211 => ed50_gk_3deg_zone(2211, 14),
        2212 => ed50_gk_3deg_zone(2212, 15),

        2213 => Ok(Crs {
            name: "ETRS89 / TM 30 NE (EPSG:2213)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(30.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2214 => Ok(Crs {
            name: "Douala 1948 / AEF West (EPSG:2214)".into(),
            datum: Datum {
                name: "Douala 1948",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(10.5)
                    .with_lat0(0.0)
                    .with_scale(0.999)
                    .with_false_easting(1_000_000.0)
                    .with_false_northing(1_000_000.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2215 => Ok(Crs {
            name: "Manoca 1962 / UTM zone 32N (EPSG:2215)".into(),
            datum: Datum {
                name: "Manoca 1962",
                ellipsoid: Ellipsoid::from_a_inv_f("Clarke 1880 IGN", 6_378_249.2, 293.466_021_293_626_5),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(32, false)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1880 IGN", 6_378_249.2, 293.466_021_293_626_5)),
            )?,
        }),

        2216 => Ok(Crs {
            name: "Qornoq 1927 / UTM zone 22N (EPSG:2216)".into(),
            datum: Datum {
                name: "Qornoq 1927",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(22, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2217 => Ok(Crs {
            name: "Qornoq 1927 / UTM zone 23N (EPSG:2217)".into(),
            datum: Datum {
                name: "Qornoq 1927",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(23, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2219 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / UTM zone 19N (EPSG:2219)".into(),
                datum,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-69.0)
                        .with_lat0(0.0)
                        .with_scale(0.9996)
                        .with_false_easting(500_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257)),
                )?,
            })
        }

        2220 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / UTM zone 20N (EPSG:2220)".into(),
                datum,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-63.0)
                        .with_lat0(0.0)
                        .with_scale(0.9996)
                        .with_false_easting(500_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257)),
                )?,
            })
        }

        2222 => us_state_plane_tm_ft(
            2222,
            "NAD83 / Arizona East (FIPS 0201) ft",
            Datum::NAD83,
            -110.1666666666667,
            31.0,
            0.9999,
            700000.0,
            0.0,
        ),

        2223 => us_state_plane_tm_ft(
            2223,
            "NAD83 / Arizona Central (FIPS 0202) ft",
            Datum::NAD83,
            -111.9166666666667,
            31.0,
            0.9999,
            700000.0,
            0.0,
        ),

        2224 => us_state_plane_tm_ft(
            2224,
            "NAD83 / Arizona West (FIPS 0203) ft",
            Datum::NAD83,
            -113.75,
            31.0,
            0.9999333333333333,
            700000.0,
            0.0,
        ),

        2225 => us_state_plane_lcc_ftus(
            2225,
            "NAD83 / California zone 1 (FIPS 0401) ftUS",
            Datum::NAD83,
            -122.0,
            40.0,
            41.66666666666666,
            39.33333333333334,
            6561666.666666666,
            1640416.666666667,
        ),

        2226 => us_state_plane_lcc_ftus(
            2226,
            "NAD83 / California zone 2 (FIPS 0402) ftUS",
            Datum::NAD83,
            -122.0,
            38.33333333333334,
            39.83333333333334,
            37.66666666666666,
            6561666.666666666,
            1640416.666666667,
        ),

        2228 => us_state_plane_lcc_ftus(
            2228,
            "NAD83 / California zone 4 (FIPS 0404) ftUS",
            Datum::NAD83,
            -119.0,
            36.0,
            37.25,
            35.33333333333334,
            6561666.666666666,
            1640416.666666667,
        ),

        2252 => us_state_plane_lcc_ft(
            2252,
            "NAD83 / Michigan Central (FIPS 2112) ft",
            Datum::NAD83,
            -84.36666666666666,
            44.18333333333333,
            45.7,
            43.31666666666667,
            19685039.37007874,
            0.0,
        ),

        2253 => us_state_plane_lcc_ft(
            2253,
            "NAD83 / Michigan South (FIPS 2113) ft",
            Datum::NAD83,
            -84.36666666666666,
            42.1,
            43.66666666666666,
            41.5,
            13123359.58005249,
            0.0,
        ),

        2254 => us_state_plane_tm_ftus(
            2254,
            "NAD83 / Mississippi East (FIPS 2301) ftUS",
            Datum::NAD83,
            -88.83333333333333,
            29.5,
            0.99995,
            984250.0,
            0.0,
        ),

        2255 => us_state_plane_tm_ftus(
            2255,
            "NAD83 / Mississippi West (FIPS 2302) ftUS",
            Datum::NAD83,
            -90.33333333333333,
            29.5,
            0.99995,
            2296583.333333333,
            0.0,
        ),

        2256 => us_state_plane_lcc_ft(
            2256,
            "NAD83 / Montana (FIPS 2500) ft",
            Datum::NAD83,
            -109.5,
            45.0,
            49.0,
            44.25,
            1968503.937007874,
            0.0,
        ),

        2257 => us_state_plane_tm_ftus(
            2257,
            "NAD83 / New Mexico East (FIPS 3001) ftUS",
            Datum::NAD83,
            -104.3333333333333,
            31.0,
            0.9999090909090909,
            541337.5,
            0.0,
        ),

        2258 => us_state_plane_tm_ftus(
            2258,
            "NAD83 / New Mexico Central (FIPS 3002) ftUS",
            Datum::NAD83,
            -106.25,
            31.0,
            0.9999,
            1640416.666666667,
            0.0,
        ),

        2259 => us_state_plane_tm_ftus(
            2259,
            "NAD83 / New Mexico West (FIPS 3003) ftUS",
            Datum::NAD83,
            -107.8333333333333,
            31.0,
            0.9999166666666667,
            2723091.666666666,
            0.0,
        ),

        2260 => us_state_plane_tm_ftus(
            2260,
            "NAD83 / New York East (FIPS 3101) ftUS",
            Datum::NAD83,
            -74.5,
            38.83333333333334,
            0.9999,
            492125.0,
            0.0,
        ),

        2261 => us_state_plane_tm_ftus(
            2261,
            "NAD83 / New York Central (FIPS 3102) ftUS",
            Datum::NAD83,
            -76.58333333333333,
            40.0,
            0.9999375,
            820208.3333333333,
            0.0,
        ),

        2262 => us_state_plane_tm_ftus(
            2262,
            "NAD83 / New York West (FIPS 3103) ftUS",
            Datum::NAD83,
            -78.58333333333333,
            40.0,
            0.9999375,
            1148291.666666667,
            0.0,
        ),

        2264 => us_state_plane_lcc_ftus(
            2264,
            "NAD83 / North Carolina (FIPS 3200) ftUS",
            Datum::NAD83,
            -79.0,
            34.33333333333334,
            36.16666666666666,
            33.75,
            2000000.002616666,
            0.0,
        ),

        2265 => us_state_plane_lcc_ft(
            2265,
            "NAD83 / North Dakota North (FIPS 3301) ft",
            Datum::NAD83,
            -100.5,
            47.43333333333333,
            48.73333333333333,
            47.0,
            1968503.937007874,
            0.0,
        ),

        2266 => us_state_plane_lcc_ft(
            2266,
            "NAD83 / North Dakota South (FIPS 3302) ft",
            Datum::NAD83,
            -100.5,
            46.18333333333333,
            47.48333333333333,
            45.66666666666666,
            1968503.937007874,
            0.0,
        ),

        2267 => us_state_plane_lcc_ftus(
            2267,
            "NAD83 / Oklahoma North (FIPS 3501) ftUS",
            Datum::NAD83,
            -98.0,
            35.56666666666667,
            36.76666666666667,
            35.0,
            1968500.0,
            0.0,
        ),

        2268 => us_state_plane_lcc_ftus(
            2268,
            "NAD83 / Oklahoma South (FIPS 3502) ftUS",
            Datum::NAD83,
            -98.0,
            33.93333333333333,
            35.23333333333333,
            33.33333333333334,
            1968500.0,
            0.0,
        ),

        2269 => us_state_plane_lcc_ft(
            2269,
            "NAD83 / Oregon North (FIPS 3601) ft",
            Datum::NAD83,
            -120.5,
            44.33333333333334,
            46.0,
            43.66666666666666,
            8202099.737532808,
            0.0,
        ),

        2270 => us_state_plane_lcc_ft(
            2270,
            "NAD83 / Oregon South (FIPS 3602) ft",
            Datum::NAD83,
            -120.5,
            42.33333333333334,
            44.0,
            41.66666666666666,
            4921259.842519685,
            0.0,
        ),

        2271 => us_state_plane_lcc_ftus(
            2271,
            "NAD83 / Pennsylvania North (FIPS 3701) ftUS",
            Datum::NAD83,
            -77.75,
            40.88333333333333,
            41.95,
            40.16666666666666,
            1968500.0,
            0.0,
        ),

        2274 => us_state_plane_lcc_ftus(
            2274,
            "NAD83 / Tennessee (FIPS 4100) ftUS",
            Datum::NAD83,
            -86.0,
            35.25,
            36.41666666666666,
            34.33333333333334,
            1968500.0,
            0.0,
        ),

        2275 => us_state_plane_lcc_ftus(
            2275,
            "NAD83 / Texas North (FIPS 4201) ftUS",
            Datum::NAD83,
            -101.5,
            34.65,
            36.18333333333333,
            34.0,
            656166.6666666665,
            3280833.333333333,
        ),

        2276 => us_state_plane_lcc_ftus(
            2276,
            "NAD83 / Texas North Central (FIPS 4202) ftUS",
            Datum::NAD83,
            -98.5,
            32.13333333333333,
            33.96666666666667,
            31.66666666666667,
            1968500.0,
            6561666.666666666,
        ),

        2277 => us_state_plane_lcc_ftus(
            2277,
            "NAD83 / Texas Central (FIPS 4203) ftUS",
            Datum::NAD83,
            -100.3333333333333,
            30.11666666666667,
            31.88333333333333,
            29.66666666666667,
            2296583.333333333,
            9842500.0,
        ),

        2278 => us_state_plane_lcc_ftus(
            2278,
            "NAD83 / Texas South Central (FIPS 4204) ftUS",
            Datum::NAD83,
            -99.0,
            28.38333333333333,
            30.28333333333333,
            27.83333333333333,
            1968500.0,
            13123333.33333333,
        ),

        2279 => us_state_plane_lcc_ftus(
            2279,
            "NAD83 / Texas South (FIPS 4205) ftUS",
            Datum::NAD83,
            -98.5,
            26.16666666666667,
            27.83333333333333,
            25.66666666666667,
            984250.0,
            16404166.66666666,
        ),

        2280 => us_state_plane_lcc_ft(
            2280,
            "NAD83 / Utah North (FIPS 4301) ft",
            Datum::NAD83,
            -111.5,
            40.71666666666667,
            41.78333333333333,
            40.33333333333334,
            1640419.947506561,
            3280839.895013123,
        ),

        2281 => us_state_plane_lcc_ft(
            2281,
            "NAD83 / Utah Central (FIPS 4302) ft",
            Datum::NAD83,
            -111.5,
            39.01666666666667,
            40.65,
            38.33333333333334,
            1640419.947506561,
            6561679.790026246,
        ),

        2282 => us_state_plane_lcc_ft(
            2282,
            "NAD83 / Utah South (FIPS 4303) ft",
            Datum::NAD83,
            -111.5,
            37.21666666666667,
            38.35,
            36.66666666666666,
            1640419.947506561,
            9842519.685039369,
        ),

        2287 => us_state_plane_lcc_ftus(
            2287,
            "NAD83 / Wisconsin North (FIPS 4801) ftUS",
            Datum::NAD83,
            -90.0,
            45.56666666666667,
            46.76666666666667,
            45.16666666666666,
            1968500.0,
            0.0,
        ),

        2288 => us_state_plane_lcc_ftus(
            2288,
            "NAD83 / Wisconsin Central (FIPS 4802) ftUS",
            Datum::NAD83,
            -90.0,
            44.25,
            45.5,
            43.83333333333334,
            1968500.0,
            0.0,
        ),

        2289 => us_state_plane_lcc_ftus(
            2289,
            "NAD83 / Wisconsin South (FIPS 4803) ftUS",
            Datum::NAD83,
            -90.0,
            42.73333333333333,
            44.06666666666667,
            42.0,
            1968500.0,
            0.0,
        ),

        2290 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / Prince Edward Island Stereographic (EPSG:2290)".into(),
                datum: datum.clone(),
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::Stereographic)
                        .with_lon0(-63.0)
                        .with_lat0(47.25)
                        .with_scale(0.999_912)
                        .with_false_easting(700_000.0)
                        .with_false_northing(400_000.0)
                        .with_ellipsoid(datum.ellipsoid.clone()),
                )?,
            })
        }

        2291 => Ok(Crs {
            name: "NAD83(CSRS) / Prince Edward Island (EPSG:2291)".into(),
            datum: Datum::NAD83_CSRS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lon0(-63.0)
                    .with_lat0(47.25)
                    .with_scale(0.999_912)
                    .with_false_easting(400_000.0)
                    .with_false_northing(800_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2292 => Ok(Crs {
            name: "NAD83(CSRS) / Prince Edward Island (EPSG:2292)".into(),
            datum: Datum::NAD83_CSRS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lon0(-63.0)
                    .with_lat0(47.25)
                    .with_scale(0.999_912)
                    .with_false_easting(400_000.0)
                    .with_false_northing(800_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2294 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / MTM Nova Scotia zone 4 (EPSG:2294)".into(),
                datum: datum.clone(),
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-61.5)
                        .with_lat0(0.0)
                        .with_scale(0.9999)
                        .with_false_easting(4_500_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(datum.ellipsoid.clone()),
                )?,
            })
        }

        2295 => {
            let datum = Datum {
                name: "ATS 1977",
                ellipsoid: Ellipsoid::from_a_inv_f("ATS 1977", 6_378_135.0, 298.257),
                transform: DatumTransform::None,
            };
            Ok(Crs {
                name: "ATS77 / MTM Nova Scotia zone 5 (EPSG:2295)".into(),
                datum: datum.clone(),
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-64.5)
                        .with_lat0(0.0)
                        .with_scale(0.9999)
                        .with_false_easting(5_500_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(datum.ellipsoid.clone()),
                )?,
            })
        }

        2308 => Ok(Crs {
            name: "Batavia / TM 109 SE (EPSG:2308)".into(),
            datum: Datum {
                name: "Batavia",
                ellipsoid: Ellipsoid::BESSEL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(109.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2309 => Ok(Crs {
            name: "WGS 84 / TM 116 SE (EPSG:2309)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(116.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2310 => Ok(Crs {
            name: "WGS 84 / TM 132 SE (EPSG:2310)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(132.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2311 => Ok(Crs {
            name: "WGS 84 / TM 6 NE (EPSG:2311)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(6.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2312 => Ok(Crs {
            name: "Garoua / UTM zone 33N (EPSG:2312)".into(),
            datum: Datum {
                name: "Garoua",
                ellipsoid: Ellipsoid::CLARKE1880_RGS,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2313 => Ok(Crs {
            name: "Kousseri / UTM zone 33N (EPSG:2313)".into(),
            datum: Datum {
                name: "Kousseri",
                ellipsoid: Ellipsoid::CLARKE1880_RGS,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2314 => Ok(Crs {
            name: "Trinidad 1903 / Trinidad Grid (EPSG:2314)".into(),
            datum: Datum {
                name: "Trinidad 1903",
                ellipsoid: Ellipsoid::from_a_inv_f("Clarke 1858 (ft Clarke)", 6_378_293.645_208_759 / 0.304_797_265_4, 294.260_676_369),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Cassini)
                    .with_lon0(-61.33333333333334)
                    .with_lat0(10.44166666666667)
                    .with_scale(1.0)
                    .with_false_easting(283_800.0)
                    .with_false_northing(214_500.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1858 (ft Clarke)", 6_378_293.645_208_759 / 0.304_797_265_4, 294.260_676_369)),
            )?,
        }),

        2315 => Ok(Crs {
            name: "Campo Inchauspe / UTM zone 19S (EPSG:2315)".into(),
            datum: Datum {
                name: "Campo Inchauspe",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-69.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2316 => Ok(Crs {
            name: "Campo Inchauspe / UTM zone 20S (EPSG:2316)".into(),
            datum: Datum {
                name: "Campo Inchauspe",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-63.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(10_000_000.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2317 => Ok(Crs {
            name: "PSAD56 / ICN Regional (EPSG:2317)".into(),
            datum: Datum {
                name: "PSAD56",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 3.0,
                    lat2: Some(9.0),
                })
                .with_lon0(-66.0)
                .with_lat0(6.0)
                .with_false_easting(1_000_000.0)
                .with_false_northing(1_000_000.0)
                .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2318 => Ok(Crs {
            name: "Ain el Abd 1970 / Aramco Lambert (EPSG:2318)".into(),
            datum: Datum {
                name: "Ain el Abd 1970",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 17.0,
                    lat2: Some(33.0),
                })
                .with_lon0(48.0)
                .with_lat0(25.08951)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2319 => Ok(Crs {
            name: "ED50 / TM27 (EPSG:2319)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(27.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2320 => Ok(Crs {
            name: "ED50 / TM30 (EPSG:2320)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(30.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2321 => Ok(Crs {
            name: "ED50 / TM33 (EPSG:2321)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(33.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2322 => Ok(Crs {
            name: "ED50 / TM36 (EPSG:2322)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(36.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2323 => Ok(Crs {
            name: "ED50 / TM39 (EPSG:2323)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(39.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2324 => Ok(Crs {
            name: "ED50 / TM42 (EPSG:2324)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(42.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2325 => Ok(Crs {
            name: "ED50 / TM45 (EPSG:2325)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(45.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2327 => Ok(Crs {
            name: "Xian 1980 / GK zone 13 (EPSG:2327)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(75.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(13_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2328 => Ok(Crs {
            name: "Xian 1980 / GK zone 14 (EPSG:2328)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(81.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(14_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2329 => Ok(Crs {
            name: "Xian 1980 / GK zone 15 (EPSG:2329)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(87.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(15_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2330 => Ok(Crs {
            name: "Xian 1980 / GK zone 16 (EPSG:2330)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(93.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(16_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2331 => Ok(Crs {
            name: "Xian 1980 / GK zone 17 (EPSG:2331)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(99.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(17_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2332 => Ok(Crs {
            name: "Xian 1980 / GK zone 18 (EPSG:2332)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(105.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(18_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        2333 => Ok(Crs {
            name: "Xian 1980 / GK zone 19 (EPSG:2333)".into(),
            datum: Datum {
                name: "Xian 1980",
                ellipsoid: Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(111.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(19_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257)),
            )?,
        }),

        3067 => Ok(Crs {
            name: "ETRS89 / TM35FIN(E,N) (EPSG:3067)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(27.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3006 => Ok(Crs {
            name: "SWEREF99 TM (EPSG:3006)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lat0(0.0)
                    .with_lon0(15.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── Germany Gauss-Krüger ─────────────────────────────────────────
        31466 => gauss_kruger(code, 6.0,  "zone 2"),
        31467 => gauss_kruger(code, 9.0,  "zone 3"),
        31468 => gauss_kruger(code, 12.0, "zone 4"),
        31469 => gauss_kruger(code, 15.0, "zone 5"),

        // ── Netherlands RD New ───────────────────────────────────────────
        28992 => Ok(Crs {
            name: "Amersfoort / RD New (EPSG:28992)".into(),
            datum: Datum::AMERSFOORT,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Stereographic)
                    .with_lat0(52.156_160_556)
                    .with_lon0(5.387_638_889)
                    .with_scale(0.9999079)
                    .with_false_easting(155_000.0)
                    .with_false_northing(463_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        // ── France RGF93 Lambert-93 ──────────────────────────────────────
        2154 => Ok(Crs {
            name: "RGF93 v1 / Lambert-93 (EPSG:2154)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 44.0,
                    lat2: Some(49.0),
                })
                .with_lat0(46.5)
                .with_lon0(3.0)
                .with_false_easting(700_000.0)
                .with_false_northing(6_600_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        31370 => Ok(Crs {
            name: "Belge 1972 / Belgian Lambert 72 (EPSG:31370)".into(),
            datum: Datum::BELGE1972,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 49.833_333_333_333,
                    lat2: Some(51.166_666_666_667),
                })
                .with_lat0(90.0)
                .with_lon0(4.367_486_666_667)
                .with_false_easting(150_000.013)
                .with_false_northing(5_400_088.438)
                .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        5514 => Ok(Crs {
            name: "S-JTSK / Krovak East North (EPSG:5514)".into(),
            datum: Datum::S_JTSK,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Krovak)
                    .with_lat0(49.5)
                    // EPSG longitudes are Greenwich-referenced; Krovak uses 24°50'E of Ferro,
                    // equivalent to 7°10'E of Greenwich.
                    .with_lon0(7.166_666_666_666_667)
                    .with_scale(0.9999)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        // ── Australia GDA94 / MGA ────────────────────────────────────────
        28349..=28356 => {
            let zone = (code - 28300) as u8; // zone 49–56
            Ok(Crs {
                name: format!("GDA94 / MGA zone {zone} (EPSG:{code})"),
                datum: Datum::GDA94,
                projection: crate::projections::Projection::new(
                    ProjectionParams::utm(zone, false)
                        .with_ellipsoid(Ellipsoid::GRS80),
                )?,
            })
        }

        // ── US State Plane (NAD83) ────────────────────────────────────────
        // California Albers
        2227 => {
            // EPSG:2227 is defined in US survey foot units.
            // Use a foot-based ellipsoid and false origin so projection outputs are in ftUS.
            let us_survey_foot = 1200.0 / 3937.0;
            let grs80_a_ft = Ellipsoid::GRS80.a / us_survey_foot;
            let grs80_inv_f = 1.0 / Ellipsoid::GRS80.f;
            us_state_plane_lcc(
                code,
                "NAD83 / California zone 3 (ftUS)",
                Datum::NAD83,
                -120.5,
                36.5,
                38.433_333,
                36.5,
                6_561_666.667,
                1_640_416.667,
            )
            .and_then(|mut crs| {
                crs.projection = crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::LambertConformalConic {
                        lat1: 36.5,
                        lat2: Some(38.433_333),
                    })
                    .with_lon0(-120.5)
                    .with_lat0(36.5)
                    .with_false_easting(6_561_666.667)
                    .with_false_northing(1_640_416.667)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f(
                        "GRS80 (ftUS)",
                        grs80_a_ft,
                        grs80_inv_f,
                    )),
                )?;
                Ok(crs)
            })
        }
        2229 => us_state_plane_lcc(code, "NAD83 / California zone 1", Datum::NAD83,
            -122.0, 40.0, 41.666_667, 40.0, 0.0, 0.0),
        2230 => us_state_plane_lcc(code, "NAD83 / California zone 2", Datum::NAD83,
            -122.0, 37.666_667, 39.833_333, 37.666_667, 2_000_000.0, 500_000.0),
        2231 => us_state_plane_lcc(code, "NAD83 / California zone 3", Datum::NAD83,
            -120.5, 36.5, 38.433_333, 36.5, 2_000_000.0, 500_000.0),
        2232 => us_state_plane_lcc(code, "NAD83 / California zone 4", Datum::NAD83,
            -119.0, 35.333_333, 37.25, 35.333_333, 2_000_000.0, 500_000.0),
        2233 => us_state_plane_lcc(code, "NAD83 / California zone 5", Datum::NAD83,
            -118.0, 33.5, 35.466_667, 33.5, 2_000_000.0, 500_000.0),
        2234 => us_state_plane_lcc(code, "NAD83 / California zone 6", Datum::NAD83,
            -116.25, 32.166_667, 33.883_333, 32.166_667, 2_000_000.0, 500_000.0),

        // Florida
        2236 => us_state_plane_tm(code, "NAD83 / Florida East", Datum::NAD83,
            -81.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        2237 => us_state_plane_tm(code, "NAD83 / Florida West", Datum::NAD83,
            -82.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        2238 => us_state_plane_lcc(code, "NAD83 / Florida North", Datum::NAD83,
            -84.5, 29.0, 30.75, 29.0, 600_000.0, 0.0),

        // Maryland
        2248 => us_state_plane_lcc(code, "NAD83 / Maryland", Datum::NAD83,
            -77.0, 37.666_667, 39.45, 37.666_667, 400_000.0, 0.0),

        // New York Long Island
        2263 => us_state_plane_lcc(code, "NAD83 / New York Long Island", Datum::NAD83,
            -74.0, 40.166_667, 41.033_333, 40.166_667, 300_000.0, 0.0),

        // Pennsylvania
        2272 => us_state_plane_lcc(code, "NAD83 / Pennsylvania North", Datum::NAD83,
            -77.75, 40.166_667, 41.95, 40.166_667, 600_000.0, 0.0),
        2273 => us_state_plane_lcc(code, "NAD83 / Pennsylvania South", Datum::NAD83,
            -77.75, 39.333_333, 40.966_667, 39.333_333, 600_000.0, 0.0),

        // Virginia
        2283 => us_state_plane_lcc(code, "NAD83 / Virginia North", Datum::NAD83,
            -78.5, 37.666_667, 39.2, 37.666_667, 3_500_000.0, 2_000_000.0),
        2284 => us_state_plane_lcc(code, "NAD83 / Virginia South", Datum::NAD83,
            -78.5, 36.333_333, 37.966_667, 36.333_333, 3_500_000.0, 1_000_000.0),

        // Washington
        2285 => us_state_plane_lcc(code, "NAD83 / Washington North", Datum::NAD83,
            -120.833_333, 47.0, 48.733_333, 47.0, 500_000.0, 0.0),
        2286 => us_state_plane_lcc(code, "NAD83 / Washington South", Datum::NAD83,
            -120.5, 45.833_333, 47.333_333, 45.833_333, 500_000.0, 0.0),

        // SPCS83 national metre codes (EPSG:26929-26998)
        26929 => us_state_plane_tm(code, "NAD83 / Alabama East", Datum::NAD83,
            -85.833_333, 30.5, 0.999_96, 200_000.0, 0.0),
        26930 => us_state_plane_tm(code, "NAD83 / Alabama West", Datum::NAD83,
            -87.5, 30.0, 0.999_933_333, 600_000.0, 0.0),
        26931 => us_state_plane_omerc(code, "NAD83 / Alaska zone 1", Datum::NAD83,
            -133.666_666_666_667, 57.0, 323.130_102_361_111, 0.9999, 5_000_000.0, -5_000_000.0),
        26932 => us_state_plane_tm(code, "NAD83 / Alaska zone 2", Datum::NAD83,
            -142.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26933 => us_state_plane_tm(code, "NAD83 / Alaska zone 3", Datum::NAD83,
            -146.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26934 => us_state_plane_tm(code, "NAD83 / Alaska zone 4", Datum::NAD83,
            -150.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26935 => us_state_plane_tm(code, "NAD83 / Alaska zone 5", Datum::NAD83,
            -154.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26936 => us_state_plane_tm(code, "NAD83 / Alaska zone 6", Datum::NAD83,
            -158.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26937 => us_state_plane_tm(code, "NAD83 / Alaska zone 7", Datum::NAD83,
            -162.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26938 => us_state_plane_tm(code, "NAD83 / Alaska zone 8", Datum::NAD83,
            -166.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26939 => us_state_plane_tm(code, "NAD83 / Alaska zone 9", Datum::NAD83,
            -170.0, 54.0, 0.999_9, 500_000.0, 0.0),
        26940 => us_state_plane_lcc(code, "NAD83 / Alaska zone 10", Datum::NAD83,
            -176.0, 53.833_333, 51.833_333, 51.0, 1_000_000.0, 0.0),
        26941 => us_state_plane_lcc(code, "NAD83 / California zone 1", Datum::NAD83,
            -122.0, 41.666_667, 40.0, 39.333_333, 2_000_000.0, 500_000.0),
        26942 => us_state_plane_lcc(code, "NAD83 / California zone 2", Datum::NAD83,
            -122.0, 39.833_333, 38.333_333, 37.666_667, 2_000_000.0, 500_000.0),
        26943 => us_state_plane_lcc(code, "NAD83 / California zone 3", Datum::NAD83,
            -120.5, 38.433_333, 37.066_667, 36.5, 2_000_000.0, 500_000.0),
        26944 => us_state_plane_lcc(code, "NAD83 / California zone 4", Datum::NAD83,
            -119.0, 37.25, 36.0, 35.333_333, 2_000_000.0, 500_000.0),
        26945 => us_state_plane_lcc(code, "NAD83 / California zone 5", Datum::NAD83,
            -118.0, 35.466_667, 34.033_333, 33.5, 2_000_000.0, 500_000.0),
        26946 => us_state_plane_lcc(code, "NAD83 / California zone 6", Datum::NAD83,
            -116.25, 33.883_333, 32.783_333, 32.166_667, 2_000_000.0, 500_000.0),
        26948 => us_state_plane_tm(code, "NAD83 / Arizona East", Datum::NAD83,
            -110.166_667, 31.0, 0.999_9, 213_360.0, 0.0),
        26949 => us_state_plane_tm(code, "NAD83 / Arizona Central", Datum::NAD83,
            -111.916_667, 31.0, 0.999_9, 213_360.0, 0.0),
        26950 => us_state_plane_tm(code, "NAD83 / Arizona West", Datum::NAD83,
            -113.75, 31.0, 0.999_933_333, 213_360.0, 0.0),
        26951 => us_state_plane_lcc(code, "NAD83 / Arkansas North", Datum::NAD83,
            -92.0, 36.233_333, 34.933_333, 34.333_333, 400_000.0, 0.0),
        26952 => us_state_plane_lcc(code, "NAD83 / Arkansas South", Datum::NAD83,
            -92.0, 34.766_667, 33.3, 32.666_667, 400_000.0, 400_000.0),
        26953 => us_state_plane_lcc(code, "NAD83 / Colorado North", Datum::NAD83,
            -105.5, 40.783_333, 39.716_667, 39.333_333, 914_401.8289, 304_800.6096),
        26954 => us_state_plane_lcc(code, "NAD83 / Colorado Central", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833_333, 914_401.8289, 304_800.6096),
        26955 => us_state_plane_lcc(code, "NAD83 / Colorado South", Datum::NAD83,
            -105.5, 38.433_333, 37.233_333, 36.666_667, 914_401.8289, 304_800.6096),
        26956 => us_state_plane_lcc(code, "NAD83 / Connecticut", Datum::NAD83,
            -72.75, 41.866_667, 41.2, 40.833_333, 304_800.6096, 152_400.3048),
        26957 => us_state_plane_tm(code, "NAD83 / Delaware", Datum::NAD83,
            -75.416_667, 38.0, 0.999_995, 200_000.0, 0.0),
        26958 => us_state_plane_tm(code, "NAD83 / Florida East", Datum::NAD83,
            -81.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        26959 => us_state_plane_tm(code, "NAD83 / Florida West", Datum::NAD83,
            -82.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        26960 => us_state_plane_lcc(code, "NAD83 / Florida North", Datum::NAD83,
            -84.5, 30.75, 29.583_333, 29.0, 600_000.0, 0.0),
        26961 => us_state_plane_tm(code, "NAD83 / Hawaii zone 1", Datum::NAD83,
            -155.5, 18.833_333, 0.999_966_667, 500_000.0, 0.0),
        26962 => us_state_plane_tm(code, "NAD83 / Hawaii zone 2", Datum::NAD83,
            -156.666_667, 20.333_333, 0.999_966_667, 500_000.0, 0.0),
        26963 => us_state_plane_tm(code, "NAD83 / Hawaii zone 3", Datum::NAD83,
            -158.0, 21.166_667, 0.999_99, 500_000.0, 0.0),
        26964 => us_state_plane_tm(code, "NAD83 / Hawaii zone 4", Datum::NAD83,
            -159.5, 21.833_333, 0.999_99, 500_000.0, 0.0),
        26965 => us_state_plane_tm(code, "NAD83 / Hawaii zone 5", Datum::NAD83,
            -160.166_667, 21.666_667, 1.0, 500_000.0, 0.0),
        26966 => us_state_plane_tm(code, "NAD83 / Georgia East", Datum::NAD83,
            -82.166_667, 30.0, 0.999_9, 200_000.0, 0.0),
        26967 => us_state_plane_tm(code, "NAD83 / Georgia West", Datum::NAD83,
            -84.166_667, 30.0, 0.999_9, 700_000.0, 0.0),
        26968 => us_state_plane_tm(code, "NAD83 / Idaho East", Datum::NAD83,
            -112.166_667, 41.666_667, 0.999_947_368, 200_000.0, 0.0),
        26969 => us_state_plane_tm(code, "NAD83 / Idaho Central", Datum::NAD83,
            -114.0, 41.666_667, 0.999_947_368, 500_000.0, 0.0),
        26970 => us_state_plane_tm(code, "NAD83 / Idaho West", Datum::NAD83,
            -115.75, 41.666_667, 0.999_933_333, 800_000.0, 0.0),
        26971 => us_state_plane_tm(code, "NAD83 / Illinois East", Datum::NAD83,
            -88.333_333, 36.666_667, 0.999_975, 300_000.0, 0.0),
        26972 => us_state_plane_tm(code, "NAD83 / Illinois West", Datum::NAD83,
            -90.166_667, 36.666_667, 0.999_941_177, 700_000.0, 0.0),
        26973 => us_state_plane_tm(code, "NAD83 / Indiana East", Datum::NAD83,
            -85.666_667, 37.5, 0.999_966_667, 100_000.0, 250_000.0),
        26974 => us_state_plane_tm(code, "NAD83 / Indiana West", Datum::NAD83,
            -87.083_333, 37.5, 0.999_966_667, 900_000.0, 250_000.0),
        26975 => us_state_plane_lcc(code, "NAD83 / Iowa North", Datum::NAD83,
            -93.5, 43.266_667, 42.066_667, 41.5, 1_500_000.0, 1_000_000.0),
        26976 => us_state_plane_lcc(code, "NAD83 / Iowa South", Datum::NAD83,
            -93.5, 41.783_333, 40.616_667, 40.0, 500_000.0, 0.0),
        26977 => us_state_plane_lcc(code, "NAD83 / Kansas North", Datum::NAD83,
            -98.0, 39.783_333, 38.716_667, 38.333_333, 400_000.0, 0.0),
        26978 => us_state_plane_lcc(code, "NAD83 / Kansas South", Datum::NAD83,
            -98.5, 38.566_667, 37.266_667, 36.666_667, 400_000.0, 400_000.0),
        26979 => us_state_plane_lcc(code, "NAD83 / Kentucky North", Datum::NAD83,
            -84.25, 37.966_667, 37.966_667, 37.5, 500_000.0, 0.0),
        26980 => us_state_plane_lcc(code, "NAD83 / Kentucky South", Datum::NAD83,
            -85.75, 37.933_333, 36.733_333, 36.333_333, 500_000.0, 500_000.0),
        26981 => us_state_plane_lcc(code, "NAD83 / Louisiana North", Datum::NAD83,
            -92.5, 32.666_667, 31.166_667, 30.5, 1_000_000.0, 0.0),
        26982 => us_state_plane_lcc(code, "NAD83 / Louisiana South", Datum::NAD83,
            -91.333_333, 30.7, 29.3, 28.5, 1_000_000.0, 0.0),
        26983 => us_state_plane_tm(code, "NAD83 / Maine East", Datum::NAD83,
            -68.5, 43.666_667, 0.999_9, 300_000.0, 0.0),
        26984 => us_state_plane_tm(code, "NAD83 / Maine West", Datum::NAD83,
            -70.166_667, 42.833_333, 0.999_966_667, 900_000.0, 0.0),
        26985 => us_state_plane_lcc(code, "NAD83 / Maryland", Datum::NAD83,
            -77.0, 39.45, 38.3, 37.666_667, 400_000.0, 0.0),
        26986 => us_state_plane_lcc(code, "NAD83 / Massachusetts Mainland", Datum::NAD83,
            -71.5, 42.683_333, 41.716_667, 41.0, 200_000.0, 750_000.0),
        26987 => us_state_plane_lcc(code, "NAD83 / Massachusetts Island", Datum::NAD83,
            -70.5, 41.483_333, 41.283_333, 41.0, 500_000.0, 0.0),
        26988 => us_state_plane_lcc(code, "NAD83 / Michigan North", Datum::NAD83,
            -87.0, 47.083_333, 45.483_333, 44.783_333, 8_000_000.0, 0.0),
        26989 => us_state_plane_lcc(code, "NAD83 / Michigan Central", Datum::NAD83,
            -84.366_667, 45.7, 44.183_333, 43.316_667, 6_000_000.0, 0.0),
        26990 => us_state_plane_lcc(code, "NAD83 / Michigan South", Datum::NAD83,
            -84.366_667, 43.666_667, 42.1, 41.5, 4_000_000.0, 0.0),
        26991 => us_state_plane_lcc(code, "NAD83 / Minnesota North", Datum::NAD83,
            -93.1, 48.633_333, 47.033_333, 46.5, 800_000.0, 100_000.0),
        26992 => us_state_plane_lcc(code, "NAD83 / Minnesota Central", Datum::NAD83,
            -94.25, 47.05, 45.616_667, 45.0, 800_000.0, 100_000.0),
        26993 => us_state_plane_lcc(code, "NAD83 / Minnesota South", Datum::NAD83,
            -94.0, 45.216_667, 43.783_333, 43.0, 800_000.0, 100_000.0),
        26994 => us_state_plane_tm(code, "NAD83 / Mississippi East", Datum::NAD83,
            -88.833_333, 29.5, 0.999_95, 300_000.0, 0.0),
        26995 => us_state_plane_tm(code, "NAD83 / Mississippi West", Datum::NAD83,
            -90.333_333, 29.5, 0.999_95, 700_000.0, 0.0),
        26996 => us_state_plane_tm(code, "NAD83 / Missouri East", Datum::NAD83,
            -90.5, 35.833_333, 0.999_933_333, 250_000.0, 0.0),
        26997 => us_state_plane_tm(code, "NAD83 / Missouri Central", Datum::NAD83,
            -92.5, 35.833_333, 0.999_933_333, 500_000.0, 0.0),
        26998 => us_state_plane_tm(code, "NAD83 / Missouri West", Datum::NAD83,
            -94.5, 36.166_667, 0.999_941_177, 850_000.0, 0.0),

        // SPCS83(NSRS2007) codes (EPSG:3465-3552; keep legacy EPSG:3502 mapping unchanged)
        3465 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alabama East", Datum::NAD83,
            -85.833333333333, 30.5, 0.99996, 200000.0, 0.0),
        3466 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alabama West", Datum::NAD83,
            -87.5, 30.0, 0.999933333, 600000.0, 0.0),
        3467 => Ok(Crs {
            name: "NAD83(NSRS2007) / Alaska Albers (EPSG:3467)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 55.0,
                    lat2: 65.0,
                })
                .with_lat0(50.0)
                .with_lon0(-154.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        3468 => us_state_plane_omerc(code, "NAD83(NSRS2007) / Alaska zone 1", Datum::NAD83,
            -133.666666666667, 57.0, 323.130102361111, 0.9999, 5000000.0, -5000000.0),
        3469 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 2", Datum::NAD83,
            -142.0, 54.0, 0.9999, 500000.0, 0.0),
        3470 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 3", Datum::NAD83,
            -146.0, 54.0, 0.9999, 500000.0, 0.0),
        3471 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 4", Datum::NAD83,
            -150.0, 54.0, 0.9999, 500000.0, 0.0),
        3472 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 5", Datum::NAD83,
            -154.0, 54.0, 0.9999, 500000.0, 0.0),
        3473 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 6", Datum::NAD83,
            -158.0, 54.0, 0.9999, 500000.0, 0.0),
        3474 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 7", Datum::NAD83,
            -162.0, 54.0, 0.9999, 500000.0, 0.0),
        3475 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 8", Datum::NAD83,
            -166.0, 54.0, 0.9999, 500000.0, 0.0),
        3476 => us_state_plane_tm(code, "NAD83(NSRS2007) / Alaska zone 9", Datum::NAD83,
            -170.0, 54.0, 0.9999, 500000.0, 0.0),
        3477 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Alaska zone 10", Datum::NAD83,
            -176.0, 53.833333333333, 51.833333333333, 51.0, 1000000.0, 0.0),
        3478 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona Central", Datum::NAD83,
            -111.916666666667, 31.0, 0.9999, 213360.0, 0.0),
        3479 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona Central (ft)", Datum::NAD83,
            -111.916666666667, 31.0, 0.9999, 700000.0, 0.0),
        3480 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona East", Datum::NAD83,
            -110.166666666667, 31.0, 0.9999, 213360.0, 0.0),
        3481 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona East (ft)", Datum::NAD83,
            -110.166666666667, 31.0, 0.9999, 700000.0, 0.0),
        3482 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona West", Datum::NAD83,
            -113.75, 31.0, 0.999933333, 213360.0, 0.0),
        3483 => us_state_plane_tm(code, "NAD83(NSRS2007) / Arizona West (ft)", Datum::NAD83,
            -113.75, 31.0, 0.999933333, 700000.0, 0.0),
        3484 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Arkansas North", Datum::NAD83,
            -92.0, 36.233333333333, 34.933333333333, 34.333333333333, 400000.0, 0.0),
        3485 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Arkansas North (ftUS)", Datum::NAD83,
            -92.0, 36.233333333333, 34.933333333333, 34.333333333333, 1312333.3333000001, 0.0),
        3486 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Arkansas South", Datum::NAD83,
            -92.0, 34.766666666667, 33.3, 32.666666666667, 400000.0, 400000.0),
        3487 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Arkansas South (ftUS)", Datum::NAD83,
            -92.0, 34.766666666667, 33.3, 32.666666666667, 1312333.3333000001, 1312333.3333000001),
        3488 => Ok(Crs {
            name: "NAD83(NSRS2007) / California Albers (EPSG:3488)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 34.0,
                    lat2: 40.5,
                })
                .with_lat0(0.0)
                .with_lon0(-120.0)
                .with_false_easting(0.0)
                .with_false_northing(-4000000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        3489 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 1", Datum::NAD83,
            -122.0, 41.666666666667, 40.0, 39.333333333333, 2000000.0, 500000.0),
        3490 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 1 (ftUS)", Datum::NAD83,
            -122.0, 41.666666666667, 40.0, 39.333333333333, 6561666.6670000004, 1640416.667),
        3491 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 2", Datum::NAD83,
            -122.0, 39.833333333333, 38.333333333333, 37.666666666667, 2000000.0, 500000.0),
        3492 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 2 (ftUS)", Datum::NAD83,
            -122.0, 39.833333333333, 38.333333333333, 37.666666666667, 6561666.6670000004, 1640416.667),
        3493 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 3", Datum::NAD83,
            -120.5, 38.433333333333, 37.066666666667, 36.5, 2000000.0, 500000.0),
        3494 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 3 (ftUS)", Datum::NAD83,
            -120.5, 38.433333333333, 37.066666666667, 36.5, 6561666.6670000004, 1640416.667),
        3495 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 4", Datum::NAD83,
            -119.0, 37.25, 36.0, 35.333333333333, 2000000.0, 500000.0),
        3496 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 4 (ftUS)", Datum::NAD83,
            -119.0, 37.25, 36.0, 35.333333333333, 6561666.6670000004, 1640416.667),
        3497 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 5", Datum::NAD83,
            -118.0, 35.466666666667, 34.033333333333, 33.5, 2000000.0, 500000.0),
        3498 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 5 (ftUS)", Datum::NAD83,
            -118.0, 35.466666666667, 34.033333333333, 33.5, 6561666.6670000004, 1640416.667),
        3499 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 6", Datum::NAD83,
            -116.25, 33.883333333333, 32.783333333333, 32.166666666667, 2000000.0, 500000.0),
        3500 => us_state_plane_lcc(code, "NAD83(NSRS2007) / California zone 6 (ftUS)", Datum::NAD83,
            -116.25, 33.883333333333, 32.783333333333, 32.166666666667, 6561666.6670000004, 1640416.667),
        3501 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Colorado Central", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833333333333, 914401.8289, 304800.6096),
        3503 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Colorado North", Datum::NAD83,
            -105.5, 40.783333333333, 39.716666666667, 39.333333333333, 914401.8289, 304800.6096),
        3504 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Colorado North (ftUS)", Datum::NAD83,
            -105.5, 40.783333333333, 39.716666666667, 39.333333333333, 3000000.0, 1000000.0),
        3505 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Colorado South", Datum::NAD83,
            -105.5, 38.433333333333, 37.233333333333, 36.666666666667, 914401.8289, 304800.6096),
        3506 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Colorado South (ftUS)", Datum::NAD83,
            -105.5, 38.433333333333, 37.233333333333, 36.666666666667, 3000000.0, 1000000.0),
        3507 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Connecticut", Datum::NAD83,
            -72.75, 41.866666666667, 41.2, 40.833333333333, 304800.6096, 152400.3048),
        3508 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Connecticut (ftUS)", Datum::NAD83,
            -72.75, 41.866666666667, 41.2, 40.833333333333, 1000000.0, 500000.0),
        3509 => us_state_plane_tm(code, "NAD83(NSRS2007) / Delaware", Datum::NAD83,
            -75.416666666667, 38.0, 0.999995, 200000.0, 0.0),
        3510 => us_state_plane_tm(code, "NAD83(NSRS2007) / Delaware (ftUS)", Datum::NAD83,
            -75.416666666667, 38.0, 0.999995, 656166.667, 0.0),
        3511 => us_state_plane_tm(code, "NAD83(NSRS2007) / Florida East", Datum::NAD83,
            -81.0, 24.333333333333, 0.999941177, 200000.0, 0.0),
        3512 => us_state_plane_tm(code, "NAD83(NSRS2007) / Florida East (ftUS)", Datum::NAD83,
            -81.0, 24.333333333333, 0.999941177, 656166.667, 0.0),
        3513 => Ok(Crs {
            name: "NAD83(NSRS2007) / Florida GDL Albers (EPSG:3513)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 24.0,
                    lat2: 31.5,
                })
                .with_lat0(24.0)
                .with_lon0(-84.0)
                .with_false_easting(400000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        3514 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Florida North", Datum::NAD83,
            -84.5, 30.75, 29.583333333333, 29.0, 600000.0, 0.0),
        3515 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Florida North (ftUS)", Datum::NAD83,
            -84.5, 30.75, 29.583333333333, 29.0, 1968500.0, 0.0),
        3516 => us_state_plane_tm(code, "NAD83(NSRS2007) / Florida West", Datum::NAD83,
            -82.0, 24.333333333333, 0.999941177, 200000.0, 0.0),
        3517 => us_state_plane_tm(code, "NAD83(NSRS2007) / Florida West (ftUS)", Datum::NAD83,
            -82.0, 24.333333333333, 0.999941177, 656166.667, 0.0),
        3518 => us_state_plane_tm(code, "NAD83(NSRS2007) / Georgia East", Datum::NAD83,
            -82.166666666667, 30.0, 0.9999, 200000.0, 0.0),
        3519 => us_state_plane_tm(code, "NAD83(NSRS2007) / Georgia East (ftUS)", Datum::NAD83,
            -82.166666666667, 30.0, 0.9999, 656166.667, 0.0),
        3520 => us_state_plane_tm(code, "NAD83(NSRS2007) / Georgia West", Datum::NAD83,
            -84.166666666667, 30.0, 0.9999, 700000.0, 0.0),
        3521 => us_state_plane_tm(code, "NAD83(NSRS2007) / Georgia West (ftUS)", Datum::NAD83,
            -84.166666666667, 30.0, 0.9999, 2296583.333, 0.0),
        3522 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho Central", Datum::NAD83,
            -114.0, 41.666666666667, 0.999947368, 500000.0, 0.0),
        3523 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho Central (ftUS)", Datum::NAD83,
            -114.0, 41.666666666667, 0.999947368, 1640416.667, 0.0),
        3524 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho East", Datum::NAD83,
            -112.166666666667, 41.666666666667, 0.999947368, 200000.0, 0.0),
        3525 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho East (ftUS)", Datum::NAD83,
            -112.166666666667, 41.666666666667, 0.999947368, 656166.667, 0.0),
        3526 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho West", Datum::NAD83,
            -115.75, 41.666666666667, 0.999933333, 800000.0, 0.0),
        3527 => us_state_plane_tm(code, "NAD83(NSRS2007) / Idaho West (ftUS)", Datum::NAD83,
            -115.75, 41.666666666667, 0.999933333, 2624666.667, 0.0),
        3528 => us_state_plane_tm(code, "NAD83(NSRS2007) / Illinois East", Datum::NAD83,
            -88.333333333333, 36.666666666667, 0.999975, 300000.0, 0.0),
        3529 => us_state_plane_tm(code, "NAD83(NSRS2007) / Illinois East (ftUS)", Datum::NAD83,
            -88.333333333333, 36.666666666667, 0.999975, 984250.0, 0.0),
        3530 => us_state_plane_tm(code, "NAD83(NSRS2007) / Illinois West", Datum::NAD83,
            -90.166666666667, 36.666666666667, 0.999941177, 700000.0, 0.0),
        3531 => us_state_plane_tm(code, "NAD83(NSRS2007) / Illinois West (ftUS)", Datum::NAD83,
            -90.166666666667, 36.666666666667, 0.999941177, 2296583.3333000001, 0.0),
        3532 => us_state_plane_tm(code, "NAD83(NSRS2007) / Indiana East", Datum::NAD83,
            -85.666666666667, 37.5, 0.999966667, 100000.0, 250000.0),
        3533 => us_state_plane_tm(code, "NAD83(NSRS2007) / Indiana East (ftUS)", Datum::NAD83,
            -85.666666666667, 37.5, 0.999966667, 328083.333, 820208.333),
        3534 => us_state_plane_tm(code, "NAD83(NSRS2007) / Indiana West", Datum::NAD83,
            -87.083333333333, 37.5, 0.999966667, 900000.0, 250000.0),
        3535 => us_state_plane_tm(code, "NAD83(NSRS2007) / Indiana West (ftUS)", Datum::NAD83,
            -87.083333333333, 37.5, 0.999966667, 2952750.0, 820208.333),
        3536 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Iowa North", Datum::NAD83,
            -93.5, 43.266666666667, 42.066666666667, 41.5, 1500000.0, 1000000.0),
        3537 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Iowa North (ftUS)", Datum::NAD83,
            -93.5, 43.266666666667, 42.066666666667, 41.5, 4921250.0, 3280833.3333000001),
        3538 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Iowa South", Datum::NAD83,
            -93.5, 41.783333333333, 40.616666666667, 40.0, 500000.0, 0.0),
        3539 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Iowa South (ftUS)", Datum::NAD83,
            -93.5, 41.783333333333, 40.616666666667, 40.0, 1640416.6667, 0.0),
        3540 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kansas North", Datum::NAD83,
            -98.0, 39.783333333333, 38.716666666667, 38.333333333333, 400000.0, 0.0),
        3541 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kansas North (ftUS)", Datum::NAD83,
            -98.0, 39.783333333333, 38.716666666667, 38.333333333333, 1312333.3333000001, 0.0),
        3542 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kansas South", Datum::NAD83,
            -98.5, 38.566666666667, 37.266666666667, 36.666666666667, 400000.0, 400000.0),
        3543 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kansas South (ftUS)", Datum::NAD83,
            -98.5, 38.566666666667, 37.266666666667, 36.666666666667, 1312333.3333000001, 1312333.3333000001),
        3544 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky North", Datum::NAD83,
            -84.25, 37.966666666667, 38.966666666667, 37.5, 500000.0, 0.0),
        3545 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky North (ftUS)", Datum::NAD83,
            -84.25, 37.966666666667, 38.966666666667, 37.5, 1640416.667, 0.0),
        3546 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky Single Zone", Datum::NAD83,
            -85.75, 37.083333333333, 38.666666666667, 36.333333333333, 1500000.0, 1000000.0),
        3547 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky Single Zone (ftUS)", Datum::NAD83,
            -85.75, 37.083333333333, 38.666666666667, 36.333333333333, 4921250.0, 3280833.333),
        3548 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky South", Datum::NAD83,
            -85.75, 37.933333333333, 36.733333333333, 36.333333333333, 500000.0, 500000.0),
        3549 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Kentucky South (ftUS)", Datum::NAD83,
            -85.75, 37.933333333333, 36.733333333333, 36.333333333333, 1640416.667, 1640416.667),
        3550 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Louisiana North", Datum::NAD83,
            -92.5, 32.666666666667, 31.166666666667, 30.5, 1000000.0, 0.0),
        3551 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Louisiana North (ftUS)", Datum::NAD83,
            -92.5, 32.666666666667, 31.166666666667, 30.5, 3280833.3333000001, 0.0),
        3552 => us_state_plane_lcc(code, "NAD83(NSRS2007) / Louisiana South", Datum::NAD83,
            -91.333333333333, 30.7, 29.3, 28.5, 1000000.0, 0.0),

        // SPCS83(NAD83(2011)) national coverage (EPSG:6355-6627, implementable subset)
        6355 => us_state_plane_tm(code, "NAD83(2011) / Alabama East", Datum::NAD83,
            -85.833333333333, 30.5, 0.99996, 200000, 0),
        6356 => us_state_plane_tm(code, "NAD83(2011) / Alabama West", Datum::NAD83,
            -87.5, 30, 0.999933333, 600000, 0),
        6393 => Ok(Crs {
            name: "NAD83(2011) / Alaska Albers (EPSG:6393)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 55.0,
                    lat2: 65.0,
                })
                .with_lat0(50.0)
                .with_lon0(-154.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        6394 => us_state_plane_omerc(code, "NAD83(2011) / Alaska zone 1", Datum::NAD83,
            -133.666666666667, 57, 323.130102361111, 0.9999, 5000000, -5000000),
        6395 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 2", Datum::NAD83,
            -142, 54, 0.9999, 500000, 0),
        6396 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 3", Datum::NAD83,
            -146, 54, 0.9999, 500000, 0),
        6397 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 4", Datum::NAD83,
            -150, 54, 0.9999, 500000, 0),
        6398 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 5", Datum::NAD83,
            -154, 54, 0.9999, 500000, 0),
        6399 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 6", Datum::NAD83,
            -158, 54, 0.9999, 500000, 0),
        6400 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 7", Datum::NAD83,
            -162, 54, 0.9999, 500000, 0),
        6401 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 8", Datum::NAD83,
            -166, 54, 0.9999, 500000, 0),
        6402 => us_state_plane_tm(code, "NAD83(2011) / Alaska zone 9", Datum::NAD83,
            -170, 54, 0.9999, 500000, 0),
        6403 => us_state_plane_lcc(code, "NAD83(2011) / Alaska zone 10", Datum::NAD83,
            -176, 53.833333333333, 51.833333333333, 51, 1000000, 0),
        6404 => us_state_plane_tm(code, "NAD83(2011) / Arizona Central", Datum::NAD83,
            -111.916666666667, 31, 0.9999, 213360, 0),
        6405 => us_state_plane_tm_ft(code, "NAD83(2011) / Arizona Central (ft)", Datum::NAD83,
            -111.916666666667, 31, 0.9999, 700000, 0),
        6406 => us_state_plane_tm(code, "NAD83(2011) / Arizona East", Datum::NAD83,
            -110.166666666667, 31, 0.9999, 213360, 0),
        6407 => us_state_plane_tm_ft(code, "NAD83(2011) / Arizona East (ft)", Datum::NAD83,
            -110.166666666667, 31, 0.9999, 700000, 0),
        6408 => us_state_plane_tm(code, "NAD83(2011) / Arizona West", Datum::NAD83,
            -113.75, 31, 0.999933333, 213360, 0),
        6409 => us_state_plane_tm_ft(code, "NAD83(2011) / Arizona West (ft)", Datum::NAD83,
            -113.75, 31, 0.999933333, 700000, 0),
        6410 => us_state_plane_lcc(code, "NAD83(2011) / Arkansas North", Datum::NAD83,
            -92, 36.233333333333, 34.933333333333, 34.333333333333, 400000, 0),
        6411 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Arkansas North (ftUS)", Datum::NAD83,
            -92, 36.233333333333, 34.933333333333, 34.333333333333, 1312333.333300000057, 0),
        6412 => us_state_plane_lcc(code, "NAD83(2011) / Arkansas South", Datum::NAD83,
            -92, 34.766666666667, 33.3, 32.666666666667, 400000, 400000),
        6413 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Arkansas South (ftUS)", Datum::NAD83,
            -92, 34.766666666667, 33.3, 32.666666666667, 1312333.333300000057, 1312333.333300000057),
        6414 => Ok(Crs {
            name: "NAD83(2011) / California Albers (EPSG:6414)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 34.0,
                    lat2: 40.5,
                })
                .with_lat0(0.0)
                .with_lon0(-120.0)
                .with_false_easting(0.0)
                .with_false_northing(-4000000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        6415 => us_state_plane_lcc(code, "NAD83(2011) / California zone 1", Datum::NAD83,
            -122, 41.666666666667, 40, 39.333333333333, 2000000, 500000),
        6416 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 1 (ftUS)", Datum::NAD83,
            -122, 41.666666666667, 40, 39.333333333333, 6561666.667000000365, 1640416.666999999899),
        6417 => us_state_plane_lcc(code, "NAD83(2011) / California zone 2", Datum::NAD83,
            -122, 39.833333333333, 38.333333333333, 37.666666666667, 2000000, 500000),
        6418 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 2 (ftUS)", Datum::NAD83,
            -122, 39.833333333333, 38.333333333333, 37.666666666667, 6561666.667000000365, 1640416.666999999899),
        6419 => us_state_plane_lcc(code, "NAD83(2011) / California zone 3", Datum::NAD83,
            -120.5, 38.433333333333, 37.066666666667, 36.5, 2000000, 500000),
        6420 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 3 (ftUS)", Datum::NAD83,
            -120.5, 38.433333333333, 37.066666666667, 36.5, 6561666.667000000365, 1640416.666999999899),
        6421 => us_state_plane_lcc(code, "NAD83(2011) / California zone 4", Datum::NAD83,
            -119, 37.25, 36, 35.333333333333, 2000000, 500000),
        6422 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 4 (ftUS)", Datum::NAD83,
            -119, 37.25, 36, 35.333333333333, 6561666.667000000365, 1640416.666999999899),
        6423 => us_state_plane_lcc(code, "NAD83(2011) / California zone 5", Datum::NAD83,
            -118, 35.466666666667, 34.033333333333, 33.5, 2000000, 500000),
        6424 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 5 (ftUS)", Datum::NAD83,
            -118, 35.466666666667, 34.033333333333, 33.5, 6561666.667000000365, 1640416.666999999899),
        6425 => us_state_plane_lcc(code, "NAD83(2011) / California zone 6", Datum::NAD83,
            -116.25, 33.883333333333, 32.783333333333, 32.166666666667, 2000000, 500000),
        6426 => us_state_plane_lcc_ftus(code, "NAD83(2011) / California zone 6 (ftUS)", Datum::NAD83,
            -116.25, 33.883333333333, 32.783333333333, 32.166666666667, 6561666.667000000365, 1640416.666999999899),
        6427 => us_state_plane_lcc(code, "NAD83(2011) / Colorado Central", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833333333333, 914401.828899999964, 304800.609600000025),
        6428 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Colorado Central (ftUS)", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833333333333, 3000000, 1000000),
        6429 => us_state_plane_lcc(code, "NAD83(2011) / Colorado North", Datum::NAD83,
            -105.5, 40.783333333333, 39.716666666667, 39.333333333333, 914401.828899999964, 304800.609600000025),
        6430 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Colorado North (ftUS)", Datum::NAD83,
            -105.5, 40.783333333333, 39.716666666667, 39.333333333333, 3000000, 1000000),
        6431 => us_state_plane_lcc(code, "NAD83(2011) / Colorado South", Datum::NAD83,
            -105.5, 38.433333333333, 37.233333333333, 36.666666666667, 914401.828899999964, 304800.609600000025),
        6432 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Colorado South (ftUS)", Datum::NAD83,
            -105.5, 38.433333333333, 37.233333333333, 36.666666666667, 3000000, 1000000),
        6433 => us_state_plane_lcc(code, "NAD83(2011) / Connecticut", Datum::NAD83,
            -72.75, 41.866666666667, 41.2, 40.833333333333, 304800.609600000025, 152400.304800000013),
        6434 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Connecticut (ftUS)", Datum::NAD83,
            -72.75, 41.866666666667, 41.2, 40.833333333333, 1000000, 500000),
        6435 => us_state_plane_tm(code, "NAD83(2011) / Delaware", Datum::NAD83,
            -75.416666666667, 38, 0.999995, 200000, 0),
        6436 => us_state_plane_tm_ftus(code, "NAD83(2011) / Delaware (ftUS)", Datum::NAD83,
            -75.416666666667, 38, 0.999995, 656166.667000000016, 0),
        6437 => us_state_plane_tm(code, "NAD83(2011) / Florida East", Datum::NAD83,
            -81, 24.333333333333, 0.999941177, 200000, 0),
        6438 => us_state_plane_tm_ftus(code, "NAD83(2011) / Florida East (ftUS)", Datum::NAD83,
            -81, 24.333333333333, 0.999941177, 656166.667000000016, 0),
        6439 => Ok(Crs {
            name: "NAD83(2011) / Florida GDL Albers (EPSG:6439)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 24.0,
                    lat2: 31.5,
                })
                .with_lat0(24.0)
                .with_lon0(-84.0)
                .with_false_easting(400000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        6440 => us_state_plane_lcc(code, "NAD83(2011) / Florida North", Datum::NAD83,
            -84.5, 30.75, 29.583333333333, 29, 600000, 0),
        6441 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Florida North (ftUS)", Datum::NAD83,
            -84.5, 30.75, 29.583333333333, 29, 1968500, 0),
        6442 => us_state_plane_tm(code, "NAD83(2011) / Florida West", Datum::NAD83,
            -82, 24.333333333333, 0.999941177, 200000, 0),
        6443 => us_state_plane_tm_ftus(code, "NAD83(2011) / Florida West (ftUS)", Datum::NAD83,
            -82, 24.333333333333, 0.999941177, 656166.667000000016, 0),
        6444 => us_state_plane_tm(code, "NAD83(2011) / Georgia East", Datum::NAD83,
            -82.166666666667, 30, 0.9999, 200000, 0),
        6445 => us_state_plane_tm_ftus(code, "NAD83(2011) / Georgia East (ftUS)", Datum::NAD83,
            -82.166666666667, 30, 0.9999, 656166.667000000016, 0),
        6446 => us_state_plane_tm(code, "NAD83(2011) / Georgia West", Datum::NAD83,
            -84.166666666667, 30, 0.9999, 700000, 0),
        6447 => us_state_plane_tm_ftus(code, "NAD83(2011) / Georgia West (ftUS)", Datum::NAD83,
            -84.166666666667, 30, 0.9999, 2296583.333000000101, 0),
        6448 => us_state_plane_tm(code, "NAD83(2011) / Idaho Central", Datum::NAD83,
            -114, 41.666666666667, 0.999947368, 500000, 0),
        6449 => us_state_plane_tm_ftus(code, "NAD83(2011) / Idaho Central (ftUS)", Datum::NAD83,
            -114, 41.666666666667, 0.999947368, 1640416.666999999899, 0),
        6450 => us_state_plane_tm(code, "NAD83(2011) / Idaho East", Datum::NAD83,
            -112.166666666667, 41.666666666667, 0.999947368, 200000, 0),
        6451 => us_state_plane_tm_ftus(code, "NAD83(2011) / Idaho East (ftUS)", Datum::NAD83,
            -112.166666666667, 41.666666666667, 0.999947368, 656166.667000000016, 0),
        6452 => us_state_plane_tm(code, "NAD83(2011) / Idaho West", Datum::NAD83,
            -115.75, 41.666666666667, 0.999933333, 800000, 0),
        6453 => us_state_plane_tm_ftus(code, "NAD83(2011) / Idaho West (ftUS)", Datum::NAD83,
            -115.75, 41.666666666667, 0.999933333, 2624666.666999999899, 0),
        6454 => us_state_plane_tm(code, "NAD83(2011) / Illinois East", Datum::NAD83,
            -88.333333333333, 36.666666666667, 0.999975, 300000, 0),
        6455 => us_state_plane_tm_ftus(code, "NAD83(2011) / Illinois East (ftUS)", Datum::NAD83,
            -88.333333333333, 36.666666666667, 0.999975, 984250, 0),
        6456 => us_state_plane_tm(code, "NAD83(2011) / Illinois West", Datum::NAD83,
            -90.166666666667, 36.666666666667, 0.999941177, 700000, 0),
        6457 => us_state_plane_tm_ftus(code, "NAD83(2011) / Illinois West (ftUS)", Datum::NAD83,
            -90.166666666667, 36.666666666667, 0.999941177, 2296583.333300000057, 0),
        6458 => us_state_plane_tm(code, "NAD83(2011) / Indiana East", Datum::NAD83,
            -85.666666666667, 37.5, 0.999966667, 100000, 250000),
        6459 => us_state_plane_tm_ftus(code, "NAD83(2011) / Indiana East (ftUS)", Datum::NAD83,
            -85.666666666667, 37.5, 0.999966667, 328083.332999999984, 820208.332999999984),
        6460 => us_state_plane_tm(code, "NAD83(2011) / Indiana West", Datum::NAD83,
            -87.083333333333, 37.5, 0.999966667, 900000, 250000),
        6461 => us_state_plane_tm_ftus(code, "NAD83(2011) / Indiana West (ftUS)", Datum::NAD83,
            -87.083333333333, 37.5, 0.999966667, 2952750, 820208.332999999984),
        6462 => us_state_plane_lcc(code, "NAD83(2011) / Iowa North", Datum::NAD83,
            -93.5, 43.266666666667, 42.066666666667, 41.5, 1500000, 1000000),
        6463 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Iowa North (ftUS)", Datum::NAD83,
            -93.5, 43.266666666667, 42.066666666667, 41.5, 4921250, 3280833.333300000057),
        6464 => us_state_plane_lcc(code, "NAD83(2011) / Iowa South", Datum::NAD83,
            -93.5, 41.783333333333, 40.616666666667, 40, 500000, 0),
        6465 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Iowa South (ftUS)", Datum::NAD83,
            -93.5, 41.783333333333, 40.616666666667, 40, 1640416.666699999943, 0),
        6466 => us_state_plane_lcc(code, "NAD83(2011) / Kansas North", Datum::NAD83,
            -98, 39.783333333333, 38.716666666667, 38.333333333333, 400000, 0),
        6467 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Kansas North (ftUS)", Datum::NAD83,
            -98, 39.783333333333, 38.716666666667, 38.333333333333, 1312333.333300000057, 0),
        6468 => us_state_plane_lcc(code, "NAD83(2011) / Kansas South", Datum::NAD83,
            -98.5, 38.566666666667, 37.266666666667, 36.666666666667, 400000, 400000),
        6469 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Kansas South (ftUS)", Datum::NAD83,
            -98.5, 38.566666666667, 37.266666666667, 36.666666666667, 1312333.333300000057, 1312333.333300000057),
        6470 => us_state_plane_lcc(code, "NAD83(2011) / Kentucky North", Datum::NAD83,
            -84.25, 37.966666666667, 38.966666666667, 37.5, 500000, 0),
        6471 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Kentucky North (ftUS)", Datum::NAD83,
            -84.25, 37.966666666667, 38.966666666667, 37.5, 1640416.666999999899, 0),
        6472 => us_state_plane_lcc(code, "NAD83(2011) / Kentucky Single Zone", Datum::NAD83,
            -85.75, 37.083333333333, 38.666666666667, 36.333333333333, 1500000, 1000000),
        6473 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Kentucky Single Zone (ftUS)", Datum::NAD83,
            -85.75, 37.083333333333, 38.666666666667, 36.333333333333, 4921250, 3280833.333000000101),
        6474 => us_state_plane_lcc(code, "NAD83(2011) / Kentucky South", Datum::NAD83,
            -85.75, 37.933333333333, 36.733333333333, 36.333333333333, 500000, 500000),
        6475 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Kentucky South (ftUS)", Datum::NAD83,
            -85.75, 37.933333333333, 36.733333333333, 36.333333333333, 1640416.666999999899, 1640416.666999999899),
        6476 => us_state_plane_lcc(code, "NAD83(2011) / Louisiana North", Datum::NAD83,
            -92.5, 32.666666666667, 31.166666666667, 30.5, 1000000, 0),
        6477 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Louisiana North (ftUS)", Datum::NAD83,
            -92.5, 32.666666666667, 31.166666666667, 30.5, 3280833.333300000057, 0),
        6478 => us_state_plane_lcc(code, "NAD83(2011) / Louisiana South", Datum::NAD83,
            -91.333333333333, 30.7, 29.3, 28.5, 1000000, 0),
        6479 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Louisiana South (ftUS)", Datum::NAD83,
            -91.333333333333, 30.7, 29.3, 28.5, 3280833.333300000057, 0),
        6480 => us_state_plane_tm(code, "NAD83(2011) / Maine CS2000 Central", Datum::NAD83,
            -69.125, 43.5, 0.99998, 500000, 0),
        6481 => us_state_plane_tm(code, "NAD83(2011) / Maine CS2000 East", Datum::NAD83,
            -67.875, 43.833333333333, 0.99998, 700000, 0),
        6482 => us_state_plane_tm(code, "NAD83(2011) / Maine CS2000 West", Datum::NAD83,
            -70.375, 42.833333333333, 0.99998, 300000, 0),
        6483 => us_state_plane_tm(code, "NAD83(2011) / Maine East", Datum::NAD83,
            -68.5, 43.666666666667, 0.9999, 300000, 0),
        6484 => us_state_plane_tm_ftus(code, "NAD83(2011) / Maine East (ftUS)", Datum::NAD83,
            -68.5, 43.666666666667, 0.9999, 984250, 0),
        6485 => us_state_plane_tm(code, "NAD83(2011) / Maine West", Datum::NAD83,
            -70.166666666667, 42.833333333333, 0.999966667, 900000, 0),
        6486 => us_state_plane_tm_ftus(code, "NAD83(2011) / Maine West (ftUS)", Datum::NAD83,
            -70.166666666667, 42.833333333333, 0.999966667, 2952750, 0),
        6487 => us_state_plane_lcc(code, "NAD83(2011) / Maryland", Datum::NAD83,
            -77, 39.45, 38.3, 37.666666666667, 400000, 0),
        6488 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Maryland (ftUS)", Datum::NAD83,
            -77, 39.45, 38.3, 37.666666666667, 1312333.333000000101, 0),
        6489 => us_state_plane_lcc(code, "NAD83(2011) / Massachusetts Island", Datum::NAD83,
            -70.5, 41.483333333333, 41.283333333333, 41, 500000, 0),
        6490 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Massachusetts Island (ftUS)", Datum::NAD83,
            -70.5, 41.483333333333, 41.283333333333, 41, 1640416.666999999899, 0),
        6491 => us_state_plane_lcc(code, "NAD83(2011) / Massachusetts Mainland", Datum::NAD83,
            -71.5, 42.683333333333, 41.716666666667, 41, 200000, 750000),
        6492 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Massachusetts Mainland (ftUS)", Datum::NAD83,
            -71.5, 42.683333333333, 41.716666666667, 41, 656166.667000000016, 2460625),
        6493 => us_state_plane_lcc(code, "NAD83(2011) / Michigan Central", Datum::NAD83,
            -84.366666666667, 45.7, 44.183333333333, 43.316666666667, 6000000, 0),
        6494 => us_state_plane_lcc_ft(code, "NAD83(2011) / Michigan Central (ft)", Datum::NAD83,
            -84.366666666667, 45.7, 44.183333333333, 43.316666666667, 19685039.370000001043, 0),
        6495 => us_state_plane_lcc(code, "NAD83(2011) / Michigan North", Datum::NAD83,
            -87, 47.083333333333, 45.483333333333, 44.783333333333, 8000000, 0),
        6496 => us_state_plane_lcc_ft(code, "NAD83(2011) / Michigan North (ft)", Datum::NAD83,
            -87, 47.083333333333, 45.483333333333, 44.783333333333, 26246719.160000000149, 0),
        6497 => us_state_plane_omerc(code, "NAD83(2011) / Michigan Oblique Mercator", Datum::NAD83,
            -86, 45.309166666667, 337.25556, 0.9996, 2546731.496, -4354009.816),
        6498 => us_state_plane_lcc(code, "NAD83(2011) / Michigan South", Datum::NAD83,
            -84.366666666667, 43.666666666667, 42.1, 41.5, 4000000, 0),
        6499 => us_state_plane_lcc_ft(code, "NAD83(2011) / Michigan South (ft)", Datum::NAD83,
            -84.366666666667, 43.666666666667, 42.1, 41.5, 13123359.580000000075, 0),
        6500 => us_state_plane_lcc(code, "NAD83(2011) / Minnesota Central", Datum::NAD83,
            -94.25, 47.05, 45.616666666667, 45, 800000, 100000),
        6501 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Minnesota Central (ftUS)", Datum::NAD83,
            -94.25, 47.05, 45.616666666667, 45, 2624666.666699999943, 328083.333299999998),
        6502 => us_state_plane_lcc(code, "NAD83(2011) / Minnesota North", Datum::NAD83,
            -93.1, 48.633333333333, 47.033333333333, 46.5, 800000, 100000),
        6503 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Minnesota North (ftUS)", Datum::NAD83,
            -93.1, 48.633333333333, 47.033333333333, 46.5, 2624666.666699999943, 328083.333299999998),
        6504 => us_state_plane_lcc(code, "NAD83(2011) / Minnesota South", Datum::NAD83,
            -94, 45.216666666667, 43.783333333333, 43, 800000, 100000),
        6505 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Minnesota South (ftUS)", Datum::NAD83,
            -94, 45.216666666667, 43.783333333333, 43, 2624666.666699999943, 328083.333299999998),
        6506 => us_state_plane_tm(code, "NAD83(2011) / Mississippi East", Datum::NAD83,
            -88.833333333333, 29.5, 0.99995, 300000, 0),
        6507 => us_state_plane_tm_ftus(code, "NAD83(2011) / Mississippi East (ftUS)", Datum::NAD83,
            -88.833333333333, 29.5, 0.99995, 984250, 0),
        6508 => us_state_plane_tm(code, "NAD83(2011) / Mississippi TM", Datum::NAD83,
            -89.75, 32.5, 0.9998335, 500000, 1300000),
        6509 => us_state_plane_tm(code, "NAD83(2011) / Mississippi West", Datum::NAD83,
            -90.333333333333, 29.5, 0.99995, 700000, 0),
        6510 => us_state_plane_tm_ftus(code, "NAD83(2011) / Mississippi West (ftUS)", Datum::NAD83,
            -90.333333333333, 29.5, 0.99995, 2296583.333000000101, 0),
        6511 => us_state_plane_tm(code, "NAD83(2011) / Missouri Central", Datum::NAD83,
            -92.5, 35.833333333333, 0.999933333, 500000, 0),
        6512 => us_state_plane_tm(code, "NAD83(2011) / Missouri East", Datum::NAD83,
            -90.5, 35.833333333333, 0.999933333, 250000, 0),
        6513 => us_state_plane_tm(code, "NAD83(2011) / Missouri West", Datum::NAD83,
            -94.5, 36.166666666667, 0.999941177, 850000, 0),
        6514 => us_state_plane_lcc(code, "NAD83(2011) / Montana", Datum::NAD83,
            -109.5, 49, 45, 44.25, 600000, 0),
        6515 => us_state_plane_lcc_ft(code, "NAD83(2011) / Montana (ft)", Datum::NAD83,
            -109.5, 49, 45, 44.25, 1968503.936999999918, 0),
        6516 => us_state_plane_lcc(code, "NAD83(2011) / Nebraska", Datum::NAD83,
            -100, 43, 40, 39.833333333333, 500000, 0),
        6517 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Nebraska (ftUS)", Datum::NAD83,
            -100, 43, 40, 39.833333333333, 1640416.666699999943, 0),
        6518 => us_state_plane_tm(code, "NAD83(2011) / Nevada Central", Datum::NAD83,
            -116.666666666667, 34.75, 0.9999, 500000, 6000000),
        6519 => us_state_plane_tm_ftus(code, "NAD83(2011) / Nevada Central (ftUS)", Datum::NAD83,
            -116.666666666667, 34.75, 0.9999, 1640416.666699999943, 19685000),
        6520 => us_state_plane_tm(code, "NAD83(2011) / Nevada East", Datum::NAD83,
            -115.583333333333, 34.75, 0.9999, 200000, 8000000),
        6521 => us_state_plane_tm_ftus(code, "NAD83(2011) / Nevada East (ftUS)", Datum::NAD83,
            -115.583333333333, 34.75, 0.9999, 656166.666699999943, 26246666.666700001806),
        6522 => us_state_plane_tm(code, "NAD83(2011) / Nevada West", Datum::NAD83,
            -118.583333333333, 34.75, 0.9999, 800000, 4000000),
        6523 => us_state_plane_tm_ftus(code, "NAD83(2011) / Nevada West (ftUS)", Datum::NAD83,
            -118.583333333333, 34.75, 0.9999, 2624666.666699999943, 13123333.333300000057),
        6524 => us_state_plane_tm(code, "NAD83(2011) / New Hampshire", Datum::NAD83,
            -71.666666666667, 42.5, 0.999966667, 300000, 0),
        6525 => us_state_plane_tm_ftus(code, "NAD83(2011) / New Hampshire (ftUS)", Datum::NAD83,
            -71.666666666667, 42.5, 0.999966667, 984250, 0),
        6526 => us_state_plane_tm(code, "NAD83(2011) / New Jersey", Datum::NAD83,
            -74.5, 38.833333333333, 0.9999, 150000, 0),
        6527 => us_state_plane_tm_ftus(code, "NAD83(2011) / New Jersey (ftUS)", Datum::NAD83,
            -74.5, 38.833333333333, 0.9999, 492125, 0),
        6528 => us_state_plane_tm(code, "NAD83(2011) / New Mexico Central", Datum::NAD83,
            -106.25, 31, 0.9999, 500000, 0),
        6529 => us_state_plane_tm_ftus(code, "NAD83(2011) / New Mexico Central (ftUS)", Datum::NAD83,
            -106.25, 31, 0.9999, 1640416.666999999899, 0),
        6530 => us_state_plane_tm(code, "NAD83(2011) / New Mexico East", Datum::NAD83,
            -104.333333333333, 31, 0.999909091, 165000, 0),
        6531 => us_state_plane_tm_ftus(code, "NAD83(2011) / New Mexico East (ftUS)", Datum::NAD83,
            -104.333333333333, 31, 0.999909091, 541337.5, 0),
        6532 => us_state_plane_tm(code, "NAD83(2011) / New Mexico West", Datum::NAD83,
            -107.833333333333, 31, 0.999916667, 830000, 0),
        6533 => us_state_plane_tm_ftus(code, "NAD83(2011) / New Mexico West (ftUS)", Datum::NAD83,
            -107.833333333333, 31, 0.999916667, 2723091.666999999899, 0),
        6534 => us_state_plane_tm(code, "NAD83(2011) / New York Central", Datum::NAD83,
            -76.583333333333, 40, 0.9999375, 250000, 0),
        6535 => us_state_plane_tm_ftus(code, "NAD83(2011) / New York Central (ftUS)", Datum::NAD83,
            -76.583333333333, 40, 0.9999375, 820208.332999999984, 0),
        6536 => us_state_plane_tm(code, "NAD83(2011) / New York East", Datum::NAD83,
            -74.5, 38.833333333333, 0.9999, 150000, 0),
        6537 => us_state_plane_tm_ftus(code, "NAD83(2011) / New York East (ftUS)", Datum::NAD83,
            -74.5, 38.833333333333, 0.9999, 492125, 0),
        6538 => us_state_plane_lcc(code, "NAD83(2011) / New York Long Island", Datum::NAD83,
            -74, 41.033333333333, 40.666666666667, 40.166666666667, 300000, 0),
        6539 => us_state_plane_lcc_ftus(code, "NAD83(2011) / New York Long Island (ftUS)", Datum::NAD83,
            -74, 41.033333333333, 40.666666666667, 40.166666666667, 984250, 0),
        6540 => us_state_plane_tm(code, "NAD83(2011) / New York West", Datum::NAD83,
            -78.583333333333, 40, 0.9999375, 350000, 0),
        6541 => us_state_plane_tm_ftus(code, "NAD83(2011) / New York West (ftUS)", Datum::NAD83,
            -78.583333333333, 40, 0.9999375, 1148291.666999999899, 0),
        6542 => us_state_plane_lcc(code, "NAD83(2011) / North Carolina", Datum::NAD83,
            -79, 36.166666666667, 34.333333333333, 33.75, 609601.219999999972, 0),
        6543 => us_state_plane_lcc_ftus(code, "NAD83(2011) / North Carolina (ftUS)", Datum::NAD83,
            -79, 36.166666666667, 34.333333333333, 33.75, 2000000, 0),
        6544 => us_state_plane_lcc(code, "NAD83(2011) / North Dakota North", Datum::NAD83,
            -100.5, 48.733333333333, 47.433333333333, 47, 600000, 0),
        6545 => us_state_plane_lcc_ft(code, "NAD83(2011) / North Dakota North (ft)", Datum::NAD83,
            -100.5, 48.733333333333, 47.433333333333, 47, 1968503.936999999918, 0),
        6546 => us_state_plane_lcc(code, "NAD83(2011) / North Dakota South", Datum::NAD83,
            -100.5, 47.483333333333, 46.183333333333, 45.666666666667, 600000, 0),
        6547 => us_state_plane_lcc_ft(code, "NAD83(2011) / North Dakota South (ft)", Datum::NAD83,
            -100.5, 47.483333333333, 46.183333333333, 45.666666666667, 1968503.936999999918, 0),
        6548 => us_state_plane_lcc(code, "NAD83(2011) / Ohio North", Datum::NAD83,
            -82.5, 41.7, 40.433333333333, 39.666666666667, 600000, 0),
        6549 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Ohio North (ftUS)", Datum::NAD83,
            -82.5, 41.7, 40.433333333333, 39.666666666667, 1968500, 0),
        6550 => us_state_plane_lcc(code, "NAD83(2011) / Ohio South", Datum::NAD83,
            -82.5, 40.033333333333, 38.733333333333, 38, 600000, 0),
        6551 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Ohio South (ftUS)", Datum::NAD83,
            -82.5, 40.033333333333, 38.733333333333, 38, 1968500, 0),
        6552 => us_state_plane_lcc(code, "NAD83(2011) / Oklahoma North", Datum::NAD83,
            -98, 36.766666666667, 35.566666666667, 35, 600000, 0),
        6553 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Oklahoma North (ftUS)", Datum::NAD83,
            -98, 36.766666666667, 35.566666666667, 35, 1968500, 0),
        6554 => us_state_plane_lcc(code, "NAD83(2011) / Oklahoma South", Datum::NAD83,
            -98, 35.233333333333, 33.933333333333, 33.333333333333, 600000, 0),
        6555 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Oklahoma South (ftUS)", Datum::NAD83,
            -98, 35.233333333333, 33.933333333333, 33.333333333333, 1968500, 0),
        6556 => us_state_plane_lcc(code, "NAD83(2011) / Oregon LCC (m)", Datum::NAD83,
            -120.5, 43, 45.5, 41.75, 400000, 0),
        6557 => us_state_plane_lcc_ft(code, "NAD83(2011) / Oregon GIC Lambert (ft)", Datum::NAD83,
            -120.5, 43, 45.5, 41.75, 1312335.958000000101, 0),
        6558 => us_state_plane_lcc(code, "NAD83(2011) / Oregon North", Datum::NAD83,
            -120.5, 46, 44.333333333333, 43.666666666667, 2500000, 0),
        6559 => us_state_plane_lcc_ft(code, "NAD83(2011) / Oregon North (ft)", Datum::NAD83,
            -120.5, 46, 44.333333333333, 43.666666666667, 8202099.737999999896, 0),
        6560 => us_state_plane_lcc(code, "NAD83(2011) / Oregon South", Datum::NAD83,
            -120.5, 44, 42.333333333333, 41.666666666667, 1500000, 0),
        6561 => us_state_plane_lcc_ft(code, "NAD83(2011) / Oregon South (ft)", Datum::NAD83,
            -120.5, 44, 42.333333333333, 41.666666666667, 4921259.843000000343, 0),
        6562 => us_state_plane_lcc(code, "NAD83(2011) / Pennsylvania North", Datum::NAD83,
            -77.75, 41.95, 40.883333333333, 40.166666666667, 600000, 0),
        6563 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Pennsylvania North (ftUS)", Datum::NAD83,
            -77.75, 41.95, 40.883333333333, 40.166666666667, 1968500, 0),
        6564 => us_state_plane_lcc(code, "NAD83(2011) / Pennsylvania South", Datum::NAD83,
            -77.75, 40.966666666667, 39.933333333333, 39.333333333333, 600000, 0),
        6565 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Pennsylvania South (ftUS)", Datum::NAD83,
            -77.75, 40.966666666667, 39.933333333333, 39.333333333333, 1968500, 0),
        6566 => us_state_plane_lcc(code, "NAD83(2011) / Puerto Rico and Virgin Is.", Datum::NAD83,
            -66.433333333333, 18.433333333333, 18.033333333333, 17.833333333333, 200000, 200000),
        6567 => us_state_plane_tm(code, "NAD83(2011) / Rhode Island", Datum::NAD83,
            -71.5, 41.083333333333, 0.99999375, 100000, 0),
        6568 => us_state_plane_tm_ftus(code, "NAD83(2011) / Rhode Island (ftUS)", Datum::NAD83,
            -71.5, 41.083333333333, 0.99999375, 328083.333299999998, 0),
        6569 => us_state_plane_lcc(code, "NAD83(2011) / South Carolina", Datum::NAD83,
            -81, 34.833333333333, 32.5, 31.833333333333, 609600, 0),
        6570 => us_state_plane_lcc_ft(code, "NAD83(2011) / South Carolina (ft)", Datum::NAD83,
            -81, 34.833333333333, 32.5, 31.833333333333, 2000000, 0),
        6571 => us_state_plane_lcc(code, "NAD83(2011) / South Dakota North", Datum::NAD83,
            -100, 45.683333333333, 44.416666666667, 43.833333333333, 600000, 0),
        6572 => us_state_plane_lcc_ftus(code, "NAD83(2011) / South Dakota North (ftUS)", Datum::NAD83,
            -100, 45.683333333333, 44.416666666667, 43.833333333333, 1968500, 0),
        6573 => us_state_plane_lcc(code, "NAD83(2011) / South Dakota South", Datum::NAD83,
            -100.333333333333, 44.4, 42.833333333333, 42.333333333333, 600000, 0),
        6574 => us_state_plane_lcc_ftus(code, "NAD83(2011) / South Dakota South (ftUS)", Datum::NAD83,
            -100.333333333333, 44.4, 42.833333333333, 42.333333333333, 1968500, 0),
        6575 => us_state_plane_lcc(code, "NAD83(2011) / Tennessee", Datum::NAD83,
            -86, 36.416666666667, 35.25, 34.333333333333, 600000, 0),
        6576 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Tennessee (ftUS)", Datum::NAD83,
            -86, 36.416666666667, 35.25, 34.333333333333, 1968500, 0),
        6577 => us_state_plane_lcc(code, "NAD83(2011) / Texas Central", Datum::NAD83,
            -100.333333333333, 31.883333333333, 30.116666666667, 29.666666666667, 700000, 3000000),
        6578 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Texas Central (ftUS)", Datum::NAD83,
            -100.333333333333, 31.883333333333, 30.116666666667, 29.666666666667, 2296583.333000000101, 9842500),
        6579 => Ok(Crs {
            name: "NAD83(2011) / Texas Centric Albers Equal Area (EPSG:6579)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 27.5,
                    lat2: 35.0,
                })
                .with_lat0(18.0)
                .with_lon0(-100.0)
                .with_false_easting(1500000.0)
                .with_false_northing(6000000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),
        6580 => us_state_plane_lcc(code, "NAD83(2011) / Texas Centric Lambert Conformal", Datum::NAD83,
            -100, 27.5, 35, 18, 1500000, 5000000),
        6581 => us_state_plane_lcc(code, "NAD83(2011) / Texas North", Datum::NAD83,
            -101.5, 36.183333333333, 34.65, 34, 200000, 1000000),
        6582 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Texas North (ftUS)", Datum::NAD83,
            -101.5, 36.183333333333, 34.65, 34, 656166.667000000016, 3280833.333000000101),
        6583 => us_state_plane_lcc(code, "NAD83(2011) / Texas North Central", Datum::NAD83,
            -98.5, 33.966666666667, 32.133333333333, 31.666666666667, 600000, 2000000),
        6584 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Texas North Central (ftUS)", Datum::NAD83,
            -98.5, 33.966666666667, 32.133333333333, 31.666666666667, 1968500, 6561666.667000000365),
        6585 => us_state_plane_lcc(code, "NAD83(2011) / Texas South", Datum::NAD83,
            -98.5, 27.833333333333, 26.166666666667, 25.666666666667, 300000, 5000000),
        6586 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Texas South (ftUS)", Datum::NAD83,
            -98.5, 27.833333333333, 26.166666666667, 25.666666666667, 984250, 16404166.666999999434),
        6587 => us_state_plane_lcc(code, "NAD83(2011) / Texas South Central", Datum::NAD83,
            -99, 30.283333333333, 28.383333333333, 27.833333333333, 600000, 4000000),
        6588 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Texas South Central (ftUS)", Datum::NAD83,
            -99, 30.283333333333, 28.383333333333, 27.833333333333, 1968500, 13123333.333000000566),
        6589 => us_state_plane_tm(code, "NAD83(2011) / Vermont", Datum::NAD83,
            -72.5, 42.5, 0.999964286, 500000, 0),
        6590 => us_state_plane_tm_ftus(code, "NAD83(2011) / Vermont (ftUS)", Datum::NAD83,
            -72.5, 42.5, 0.999964286, 1640416.666699999943, 0),
        6591 => us_state_plane_lcc(code, "NAD83(2011) / Virginia Lambert", Datum::NAD83,
            -79.5, 37, 39.5, 36, 0, 0),
        6592 => us_state_plane_lcc(code, "NAD83(2011) / Virginia North", Datum::NAD83,
            -78.5, 39.2, 38.033333333333, 37.666666666667, 3500000, 2000000),
        6593 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Virginia North (ftUS)", Datum::NAD83,
            -78.5, 39.2, 38.033333333333, 37.666666666667, 11482916.666999999434, 6561666.667000000365),
        6594 => us_state_plane_lcc(code, "NAD83(2011) / Virginia South", Datum::NAD83,
            -78.5, 37.966666666667, 36.766666666667, 36.333333333333, 3500000, 1000000),
        6595 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Virginia South (ftUS)", Datum::NAD83,
            -78.5, 37.966666666667, 36.766666666667, 36.333333333333, 11482916.666999999434, 3280833.333000000101),
        6596 => us_state_plane_lcc(code, "NAD83(2011) / Washington North", Datum::NAD83,
            -120.833333333333, 48.733333333333, 47.5, 47, 500000, 0),
        6597 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Washington North (ftUS)", Datum::NAD83,
            -120.833333333333, 48.733333333333, 47.5, 47, 1640416.666999999899, 0),
        6598 => us_state_plane_lcc(code, "NAD83(2011) / Washington South", Datum::NAD83,
            -120.5, 47.333333333333, 45.833333333333, 45.333333333333, 500000, 0),
        6599 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Washington South (ftUS)", Datum::NAD83,
            -120.5, 47.333333333333, 45.833333333333, 45.333333333333, 1640416.666999999899, 0),
        6600 => us_state_plane_lcc(code, "NAD83(2011) / West Virginia North", Datum::NAD83,
            -79.5, 40.25, 39, 38.5, 600000, 0),
        6601 => us_state_plane_lcc_ftus(code, "NAD83(2011) / West Virginia North (ftUS)", Datum::NAD83,
            -79.5, 40.25, 39, 38.5, 1968500, 0),
        6602 => us_state_plane_lcc(code, "NAD83(2011) / West Virginia South", Datum::NAD83,
            -81, 38.883333333333, 37.483333333333, 37, 600000, 0),
        6603 => us_state_plane_lcc_ftus(code, "NAD83(2011) / West Virginia South (ftUS)", Datum::NAD83,
            -81, 38.883333333333, 37.483333333333, 37, 1968500, 0),
        6604 => us_state_plane_lcc(code, "NAD83(2011) / Wisconsin Central", Datum::NAD83,
            -90, 45.5, 44.25, 43.833333333333, 600000, 0),
        6605 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Wisconsin Central (ftUS)", Datum::NAD83,
            -90, 45.5, 44.25, 43.833333333333, 1968500, 0),
        6606 => us_state_plane_lcc(code, "NAD83(2011) / Wisconsin North", Datum::NAD83,
            -90, 46.766666666667, 45.566666666667, 45.166666666667, 600000, 0),
        6607 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Wisconsin North (ftUS)", Datum::NAD83,
            -90, 46.766666666667, 45.566666666667, 45.166666666667, 1968500, 0),
        6608 => us_state_plane_lcc(code, "NAD83(2011) / Wisconsin South", Datum::NAD83,
            -90, 44.066666666667, 42.733333333333, 42, 600000, 0),
        6609 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Wisconsin South (ftUS)", Datum::NAD83,
            -90, 44.066666666667, 42.733333333333, 42, 1968500, 0),
        6610 => us_state_plane_tm(code, "NAD83(2011) / Wisconsin Transverse Mercator", Datum::NAD83,
            -90, 0, 0.9996, 520000, -4480000),
        6611 => us_state_plane_tm(code, "NAD83(2011) / Wyoming East", Datum::NAD83,
            -105.166666666667, 40.5, 0.9999375, 200000, 0),
        6612 => us_state_plane_tm_ftus(code, "NAD83(2011) / Wyoming East (ftUS)", Datum::NAD83,
            -105.166666666667, 40.5, 0.9999375, 656166.666699999943, 0),
        6613 => us_state_plane_tm(code, "NAD83(2011) / Wyoming East Central", Datum::NAD83,
            -107.333333333333, 40.5, 0.9999375, 400000, 100000),
        6614 => us_state_plane_tm_ftus(code, "NAD83(2011) / Wyoming East Central (ftUS)", Datum::NAD83,
            -107.333333333333, 40.5, 0.9999375, 1312333.333300000057, 328083.333299999998),
        6615 => us_state_plane_tm(code, "NAD83(2011) / Wyoming West", Datum::NAD83,
            -110.083333333333, 40.5, 0.9999375, 800000, 100000),
        6616 => us_state_plane_tm_ftus(code, "NAD83(2011) / Wyoming West (ftUS)", Datum::NAD83,
            -110.083333333333, 40.5, 0.9999375, 2624666.666699999943, 328083.333299999998),
        6617 => us_state_plane_tm(code, "NAD83(2011) / Wyoming West Central", Datum::NAD83,
            -108.75, 40.5, 0.9999375, 600000, 0),
        6618 => us_state_plane_tm_ftus(code, "NAD83(2011) / Wyoming West Central (ftUS)", Datum::NAD83,
            -108.75, 40.5, 0.9999375, 1968500, 0),
        6619 => us_state_plane_lcc(code, "NAD83(2011) / Utah Central", Datum::NAD83,
            -111.5, 40.65, 39.016666666667, 38.333333333333, 500000, 2000000),
        6620 => us_state_plane_lcc(code, "NAD83(2011) / Utah North", Datum::NAD83,
            -111.5, 41.783333333333, 40.716666666667, 40.333333333333, 500000, 1000000),
        6621 => us_state_plane_lcc(code, "NAD83(2011) / Utah South", Datum::NAD83,
            -111.5, 38.35, 37.216666666667, 36.666666666667, 500000, 3000000),
        6625 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Utah Central (ftUS)", Datum::NAD83,
            -111.5, 40.65, 39.016666666667, 38.333333333333, 1640416.666699999943, 6561666.666699999943),
        6626 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Utah North (ftUS)", Datum::NAD83,
            -111.5, 41.783333333333, 40.716666666667, 40.333333333333, 1640416.666699999943, 3280833.333300000057),
        6627 => us_state_plane_lcc_ftus(code, "NAD83(2011) / Utah South (ftUS)", Datum::NAD83,
            -111.5, 38.35, 37.216666666667, 36.666666666667, 1640416.666699999943, 9842500),

        // SPCS83(HARN) national metre codes (EPSG:2759-2866)
        2759 => us_state_plane_tm(code, "NAD83(HARN) / Alabama East", Datum::NAD83,
            -85.833_333, 30.5, 0.999_96, 200_000.0, 0.0),
        2760 => us_state_plane_tm(code, "NAD83(HARN) / Alabama West", Datum::NAD83,
            -87.5, 30.0, 0.999_933_333, 600_000.0, 0.0),
        2761 => us_state_plane_tm(code, "NAD83(HARN) / Arizona East", Datum::NAD83,
            -110.166_667, 31.0, 0.999_9, 213_360.0, 0.0),
        2762 => us_state_plane_tm(code, "NAD83(HARN) / Arizona Central", Datum::NAD83,
            -111.916_667, 31.0, 0.999_9, 213_360.0, 0.0),
        2763 => us_state_plane_tm(code, "NAD83(HARN) / Arizona West", Datum::NAD83,
            -113.75, 31.0, 0.999_933_333, 213_360.0, 0.0),
        2764 => us_state_plane_lcc(code, "NAD83(HARN) / Arkansas North", Datum::NAD83,
            -92.0, 36.233_333, 34.933_333, 34.333_333, 400_000.0, 0.0),
        2765 => us_state_plane_lcc(code, "NAD83(HARN) / Arkansas South", Datum::NAD83,
            -92.0, 34.766_667, 33.3, 32.666_667, 400_000.0, 400_000.0),
        2766 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 1", Datum::NAD83,
            -122.0, 41.666_667, 40.0, 39.333_333, 2_000_000.0, 500_000.0),
        2767 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 2", Datum::NAD83,
            -122.0, 39.833_333, 38.333_333, 37.666_667, 2_000_000.0, 500_000.0),
        2768 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 3", Datum::NAD83,
            -120.5, 38.433_333, 37.066_667, 36.5, 2_000_000.0, 500_000.0),
        2769 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 4", Datum::NAD83,
            -119.0, 37.25, 36.0, 35.333_333, 2_000_000.0, 500_000.0),
        2770 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 5", Datum::NAD83,
            -118.0, 35.466_667, 34.033_333, 33.5, 2_000_000.0, 500_000.0),
        2771 => us_state_plane_lcc(code, "NAD83(HARN) / California zone 6", Datum::NAD83,
            -116.25, 33.883_333, 32.783_333, 32.166_667, 2_000_000.0, 500_000.0),
        2772 => us_state_plane_lcc(code, "NAD83(HARN) / Colorado North", Datum::NAD83,
            -105.5, 40.783_333, 39.716_667, 39.333_333, 914_401.8289, 304_800.6096),
        2773 => us_state_plane_lcc(code, "NAD83(HARN) / Colorado Central", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833_333, 914_401.8289, 304_800.6096),
        2774 => us_state_plane_lcc(code, "NAD83(HARN) / Colorado South", Datum::NAD83,
            -105.5, 38.433_333, 37.233_333, 36.666_667, 914_401.8289, 304_800.6096),
        2775 => us_state_plane_lcc(code, "NAD83(HARN) / Connecticut", Datum::NAD83,
            -72.75, 41.866_667, 41.2, 40.833_333, 304_800.6096, 152_400.3048),
        2776 => us_state_plane_tm(code, "NAD83(HARN) / Delaware", Datum::NAD83,
            -75.416_667, 38.0, 0.999_995, 200_000.0, 0.0),
        2777 => us_state_plane_tm(code, "NAD83(HARN) / Florida East", Datum::NAD83,
            -81.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        2778 => us_state_plane_tm(code, "NAD83(HARN) / Florida West", Datum::NAD83,
            -82.0, 24.333_333, 0.999_941_177, 200_000.0, 0.0),
        2779 => us_state_plane_lcc(code, "NAD83(HARN) / Florida North", Datum::NAD83,
            -84.5, 30.75, 29.583_333, 29.0, 600_000.0, 0.0),
        2780 => us_state_plane_tm(code, "NAD83(HARN) / Georgia East", Datum::NAD83,
            -82.166_667, 30.0, 0.999_9, 200_000.0, 0.0),
        2781 => us_state_plane_tm(code, "NAD83(HARN) / Georgia West", Datum::NAD83,
            -84.166_667, 30.0, 0.999_9, 700_000.0, 0.0),
        2782 => us_state_plane_tm(code, "NAD83(HARN) / Hawaii zone 1", Datum::NAD83,
            -155.5, 18.833_333, 0.999_966_667, 500_000.0, 0.0),
        2783 => us_state_plane_tm(code, "NAD83(HARN) / Hawaii zone 2", Datum::NAD83,
            -156.666_667, 20.333_333, 0.999_966_667, 500_000.0, 0.0),
        2784 => us_state_plane_tm(code, "NAD83(HARN) / Hawaii zone 3", Datum::NAD83,
            -158.0, 21.166_667, 0.999_99, 500_000.0, 0.0),
        2785 => us_state_plane_tm(code, "NAD83(HARN) / Hawaii zone 4", Datum::NAD83,
            -159.5, 21.833_333, 0.999_99, 500_000.0, 0.0),
        2786 => us_state_plane_tm(code, "NAD83(HARN) / Hawaii zone 5", Datum::NAD83,
            -160.166_667, 21.666_667, 1.0, 500_000.0, 0.0),
        2787 => us_state_plane_tm(code, "NAD83(HARN) / Idaho East", Datum::NAD83,
            -112.166_667, 41.666_667, 0.999_947_368, 200_000.0, 0.0),
        2788 => us_state_plane_tm(code, "NAD83(HARN) / Idaho Central", Datum::NAD83,
            -114.0, 41.666_667, 0.999_947_368, 500_000.0, 0.0),
        2789 => us_state_plane_tm(code, "NAD83(HARN) / Idaho West", Datum::NAD83,
            -115.75, 41.666_667, 0.999_933_333, 800_000.0, 0.0),
        2790 => us_state_plane_tm(code, "NAD83(HARN) / Illinois East", Datum::NAD83,
            -88.333_333, 36.666_667, 0.999_975, 300_000.0, 0.0),
        2791 => us_state_plane_tm(code, "NAD83(HARN) / Illinois West", Datum::NAD83,
            -90.166_667, 36.666_667, 0.999_941_177, 700_000.0, 0.0),
        2792 => us_state_plane_tm(code, "NAD83(HARN) / Indiana East", Datum::NAD83,
            -85.666_667, 37.5, 0.999_966_667, 100_000.0, 250_000.0),
        2793 => us_state_plane_tm(code, "NAD83(HARN) / Indiana West", Datum::NAD83,
            -87.083_333, 37.5, 0.999_966_667, 900_000.0, 250_000.0),
        2794 => us_state_plane_lcc(code, "NAD83(HARN) / Iowa North", Datum::NAD83,
            -93.5, 43.266_667, 42.066_667, 41.5, 1_500_000.0, 1_000_000.0),
        2795 => us_state_plane_lcc(code, "NAD83(HARN) / Iowa South", Datum::NAD83,
            -93.5, 41.783_333, 40.616_667, 40.0, 500_000.0, 0.0),
        2796 => us_state_plane_lcc(code, "NAD83(HARN) / Kansas North", Datum::NAD83,
            -98.0, 39.783_333, 38.716_667, 38.333_333, 400_000.0, 0.0),
        2797 => us_state_plane_lcc(code, "NAD83(HARN) / Kansas South", Datum::NAD83,
            -98.5, 38.566_667, 37.266_667, 36.666_667, 400_000.0, 400_000.0),
        2798 => us_state_plane_lcc(code, "NAD83(HARN) / Kentucky North", Datum::NAD83,
            -84.25, 37.966_667, 38.966_667, 37.5, 500_000.0, 0.0),
        2799 => us_state_plane_lcc(code, "NAD83(HARN) / Kentucky South", Datum::NAD83,
            -85.75, 37.933_333, 36.733_333, 36.333_333, 500_000.0, 500_000.0),
        2800 => us_state_plane_lcc(code, "NAD83(HARN) / Louisiana North", Datum::NAD83,
            -92.5, 32.666_667, 31.166_667, 30.5, 1_000_000.0, 0.0),
        2801 => us_state_plane_lcc(code, "NAD83(HARN) / Louisiana South", Datum::NAD83,
            -91.333_333, 30.7, 29.3, 28.5, 1_000_000.0, 0.0),
        2802 => us_state_plane_tm(code, "NAD83(HARN) / Maine East", Datum::NAD83,
            -68.5, 43.666_667, 0.999_9, 300_000.0, 0.0),
        2803 => us_state_plane_tm(code, "NAD83(HARN) / Maine West", Datum::NAD83,
            -70.166_667, 42.833_333, 0.999_966_667, 900_000.0, 0.0),
        2804 => us_state_plane_lcc(code, "NAD83(HARN) / Maryland", Datum::NAD83,
            -77.0, 39.45, 38.3, 37.666_667, 400_000.0, 0.0),
        2805 => us_state_plane_lcc(code, "NAD83(HARN) / Massachusetts Mainland", Datum::NAD83,
            -71.5, 42.683_333, 41.716_667, 41.0, 200_000.0, 750_000.0),
        2806 => us_state_plane_lcc(code, "NAD83(HARN) / Massachusetts Island", Datum::NAD83,
            -70.5, 41.483_333, 41.283_333, 41.0, 500_000.0, 0.0),
        2807 => us_state_plane_lcc(code, "NAD83(HARN) / Michigan North", Datum::NAD83,
            -87.0, 47.083_333, 45.483_333, 44.783_333, 8_000_000.0, 0.0),
        2808 => us_state_plane_lcc(code, "NAD83(HARN) / Michigan Central", Datum::NAD83,
            -84.366_667, 45.7, 44.183_333, 43.316_667, 6_000_000.0, 0.0),
        2809 => us_state_plane_lcc(code, "NAD83(HARN) / Michigan South", Datum::NAD83,
            -84.366_667, 43.666_667, 42.1, 41.5, 4_000_000.0, 0.0),
        2810 => us_state_plane_lcc(code, "NAD83(HARN) / Minnesota North", Datum::NAD83,
            -93.1, 48.633_333, 47.033_333, 46.5, 800_000.0, 100_000.0),
        2811 => us_state_plane_lcc(code, "NAD83(HARN) / Minnesota Central", Datum::NAD83,
            -94.25, 47.05, 45.616_667, 45.0, 800_000.0, 100_000.0),
        2812 => us_state_plane_lcc(code, "NAD83(HARN) / Minnesota South", Datum::NAD83,
            -94.0, 45.216_667, 43.783_333, 43.0, 800_000.0, 100_000.0),
        2813 => us_state_plane_tm(code, "NAD83(HARN) / Mississippi East", Datum::NAD83,
            -88.833_333, 29.5, 0.999_95, 300_000.0, 0.0),
        2814 => us_state_plane_tm(code, "NAD83(HARN) / Mississippi West", Datum::NAD83,
            -90.333_333, 29.5, 0.999_95, 700_000.0, 0.0),
        2815 => us_state_plane_tm(code, "NAD83(HARN) / Missouri East", Datum::NAD83,
            -90.5, 35.833_333, 0.999_933_333, 250_000.0, 0.0),
        2816 => us_state_plane_tm(code, "NAD83(HARN) / Missouri Central", Datum::NAD83,
            -92.5, 35.833_333, 0.999_933_333, 500_000.0, 0.0),
        2817 => us_state_plane_tm(code, "NAD83(HARN) / Missouri West", Datum::NAD83,
            -94.5, 36.166_667, 0.999_941_177, 850_000.0, 0.0),
        2818 => us_state_plane_lcc(code, "NAD83(HARN) / Montana", Datum::NAD83,
            -109.5, 49.0, 45.0, 44.25, 600_000.0, 0.0),
        2819 => us_state_plane_lcc(code, "NAD83(HARN) / Nebraska", Datum::NAD83,
            -100.0, 43.0, 40.0, 39.833_333, 500_000.0, 0.0),
        2820 => us_state_plane_tm(code, "NAD83(HARN) / Nevada East", Datum::NAD83,
            -115.583_333, 34.75, 0.999_9, 200_000.0, 8_000_000.0),
        2821 => us_state_plane_tm(code, "NAD83(HARN) / Nevada Central", Datum::NAD83,
            -116.666_667, 34.75, 0.999_9, 500_000.0, 6_000_000.0),
        2822 => us_state_plane_tm(code, "NAD83(HARN) / Nevada West", Datum::NAD83,
            -118.583_333, 34.75, 0.999_9, 800_000.0, 4_000_000.0),
        2823 => us_state_plane_tm(code, "NAD83(HARN) / New Hampshire", Datum::NAD83,
            -71.666_667, 42.5, 0.999_966_667, 300_000.0, 0.0),
        2824 => us_state_plane_tm(code, "NAD83(HARN) / New Jersey", Datum::NAD83,
            -74.5, 38.833_333, 0.999_9, 150_000.0, 0.0),
        2825 => us_state_plane_tm(code, "NAD83(HARN) / New Mexico East", Datum::NAD83,
            -104.333_333, 31.0, 0.999_909_091, 165_000.0, 0.0),
        2826 => us_state_plane_tm(code, "NAD83(HARN) / New Mexico Central", Datum::NAD83,
            -106.25, 31.0, 0.999_9, 500_000.0, 0.0),
        2827 => us_state_plane_tm(code, "NAD83(HARN) / New Mexico West", Datum::NAD83,
            -107.833_333, 31.0, 0.999_916_667, 830_000.0, 0.0),
        2828 => us_state_plane_tm(code, "NAD83(HARN) / New York East", Datum::NAD83,
            -74.5, 38.833_333, 0.999_9, 150_000.0, 0.0),
        2829 => us_state_plane_tm(code, "NAD83(HARN) / New York Central", Datum::NAD83,
            -76.583_333, 40.0, 0.999_937_5, 250_000.0, 0.0),
        2830 => us_state_plane_tm(code, "NAD83(HARN) / New York West", Datum::NAD83,
            -78.583_333, 40.0, 0.999_937_5, 350_000.0, 0.0),
        2831 => us_state_plane_lcc(code, "NAD83(HARN) / New York Long Island", Datum::NAD83,
            -74.0, 41.033_333, 40.666_667, 40.166_667, 300_000.0, 0.0),
        2832 => us_state_plane_lcc(code, "NAD83(HARN) / North Dakota North", Datum::NAD83,
            -100.5, 48.733_333, 47.433_333, 47.0, 600_000.0, 0.0),
        2833 => us_state_plane_lcc(code, "NAD83(HARN) / North Dakota South", Datum::NAD83,
            -100.5, 47.483_333, 46.183_333, 45.666_667, 600_000.0, 0.0),
        2834 => us_state_plane_lcc(code, "NAD83(HARN) / Ohio North", Datum::NAD83,
            -82.5, 41.7, 40.433_333, 39.666_667, 600_000.0, 0.0),
        2835 => us_state_plane_lcc(code, "NAD83(HARN) / Ohio South", Datum::NAD83,
            -82.5, 40.033_333, 38.733_333, 38.0, 600_000.0, 0.0),
        2836 => us_state_plane_lcc(code, "NAD83(HARN) / Oklahoma North", Datum::NAD83,
            -98.0, 36.766_667, 35.566_667, 35.0, 600_000.0, 0.0),
        2837 => us_state_plane_lcc(code, "NAD83(HARN) / Oklahoma South", Datum::NAD83,
            -98.0, 35.233_333, 33.933_333, 33.333_333, 600_000.0, 0.0),
        2838 => us_state_plane_lcc(code, "NAD83(HARN) / Oregon North", Datum::NAD83,
            -120.5, 46.0, 44.333_333, 43.666_667, 2_500_000.0, 0.0),
        2839 => us_state_plane_lcc(code, "NAD83(HARN) / Oregon South", Datum::NAD83,
            -120.5, 44.0, 42.333_333, 41.666_667, 1_500_000.0, 0.0),
        2840 => us_state_plane_tm(code, "NAD83(HARN) / Rhode Island", Datum::NAD83,
            -71.5, 41.083_333, 0.999_993_75, 100_000.0, 0.0),
        2841 => us_state_plane_lcc(code, "NAD83(HARN) / South Dakota North", Datum::NAD83,
            -100.0, 45.683_333, 44.416_667, 43.833_333, 600_000.0, 0.0),
        2842 => us_state_plane_lcc(code, "NAD83(HARN) / South Dakota South", Datum::NAD83,
            -100.333_333, 44.4, 42.833_333, 42.333_333, 600_000.0, 0.0),
        2843 => us_state_plane_lcc(code, "NAD83(HARN) / Tennessee", Datum::NAD83,
            -86.0, 36.416_667, 35.25, 34.333_333, 600_000.0, 0.0),
        2844 => us_state_plane_lcc(code, "NAD83(HARN) / Texas North", Datum::NAD83,
            -101.5, 36.183_333, 34.65, 34.0, 200_000.0, 1_000_000.0),
        2845 => us_state_plane_lcc(code, "NAD83(HARN) / Texas North Central", Datum::NAD83,
            -98.5, 33.966_667, 32.133_333, 31.666_667, 600_000.0, 2_000_000.0),
        2846 => us_state_plane_lcc(code, "NAD83(HARN) / Texas Central", Datum::NAD83,
            -100.333_333, 31.883_333, 30.116_667, 29.666_667, 700_000.0, 3_000_000.0),
        2847 => us_state_plane_lcc(code, "NAD83(HARN) / Texas South Central", Datum::NAD83,
            -99.0, 30.283_333, 28.383_333, 27.833_333, 600_000.0, 4_000_000.0),
        2848 => us_state_plane_lcc(code, "NAD83(HARN) / Texas South", Datum::NAD83,
            -98.5, 27.833_333, 26.166_667, 25.666_667, 300_000.0, 5_000_000.0),
        2849 => us_state_plane_lcc(code, "NAD83(HARN) / Utah North", Datum::NAD83,
            -111.5, 41.783_333, 40.716_667, 40.333_333, 500_000.0, 1_000_000.0),
        2850 => us_state_plane_lcc(code, "NAD83(HARN) / Utah Central", Datum::NAD83,
            -111.5, 40.65, 39.016_667, 38.333_333, 500_000.0, 2_000_000.0),
        2851 => us_state_plane_lcc(code, "NAD83(HARN) / Utah South", Datum::NAD83,
            -111.5, 38.35, 37.216_667, 36.666_667, 500_000.0, 3_000_000.0),
        2852 => us_state_plane_tm(code, "NAD83(HARN) / Vermont", Datum::NAD83,
            -72.5, 42.5, 0.999_964_286, 500_000.0, 0.0),
        2853 => us_state_plane_lcc(code, "NAD83(HARN) / Virginia North", Datum::NAD83,
            -78.5, 39.2, 38.033_333, 37.666_667, 3_500_000.0, 2_000_000.0),
        2854 => us_state_plane_lcc(code, "NAD83(HARN) / Virginia South", Datum::NAD83,
            -78.5, 37.966_667, 36.766_667, 36.333_333, 3_500_000.0, 1_000_000.0),
        2855 => us_state_plane_lcc(code, "NAD83(HARN) / Washington North", Datum::NAD83,
            -120.833_333, 48.733_333, 47.5, 47.0, 500_000.0, 0.0),
        2856 => us_state_plane_lcc(code, "NAD83(HARN) / Washington South", Datum::NAD83,
            -120.5, 47.333_333, 45.833_333, 45.333_333, 500_000.0, 0.0),
        2857 => us_state_plane_lcc(code, "NAD83(HARN) / West Virginia North", Datum::NAD83,
            -79.5, 40.25, 39.0, 38.5, 600_000.0, 0.0),
        2858 => us_state_plane_lcc(code, "NAD83(HARN) / West Virginia South", Datum::NAD83,
            -81.0, 38.883_333, 37.483_333, 37.0, 600_000.0, 0.0),
        2859 => us_state_plane_lcc(code, "NAD83(HARN) / Wisconsin North", Datum::NAD83,
            -90.0, 46.766_667, 45.566_667, 45.166_667, 600_000.0, 0.0),
        2860 => us_state_plane_lcc(code, "NAD83(HARN) / Wisconsin Central", Datum::NAD83,
            -90.0, 45.5, 44.25, 43.833_333, 600_000.0, 0.0),
        2861 => us_state_plane_lcc(code, "NAD83(HARN) / Wisconsin South", Datum::NAD83,
            -90.0, 44.066_667, 42.733_333, 42.0, 600_000.0, 0.0),
        2862 => us_state_plane_tm(code, "NAD83(HARN) / Wyoming East", Datum::NAD83,
            -105.166_667, 40.5, 0.999_937_5, 200_000.0, 0.0),
        2863 => us_state_plane_tm(code, "NAD83(HARN) / Wyoming East Central", Datum::NAD83,
            -107.333_333, 40.5, 0.999_937_5, 400_000.0, 100_000.0),
        2864 => us_state_plane_tm(code, "NAD83(HARN) / Wyoming West Central", Datum::NAD83,
            -108.75, 40.5, 0.999_937_5, 600_000.0, 0.0),
        2865 => us_state_plane_tm(code, "NAD83(HARN) / Wyoming West", Datum::NAD83,
            -110.083_333, 40.5, 0.999_937_5, 800_000.0, 100_000.0),
        2866 => us_state_plane_lcc(code, "NAD83(HARN) / Puerto Rico and Virgin Is.", Datum::NAD83,
            -66.433_333, 18.433_333, 18.033_333, 17.833_333, 200_000.0, 200_000.0),

        3502 => us_state_plane_lcc_ftus(code, "NAD83(NSRS2007) / Colorado Central (ftUS)", Datum::NAD83,
            -105.5, 39.75, 38.45, 37.833_333_333_333, 914_401.828_803_658, 304_800.609_601_219),

        3338 => us_state_plane_tm(code, "NAD83 / Alaska zone 1", Datum::NAD83,
            -133.666_667, 54.0, 0.999_9, 5_000_000.0, -4_000_000.0),

        // ── Swiss ────────────────────────────────────────────────────────
        21781 => Ok(Crs {
            name: "CH1903 / LV03 (EPSG:21781)".into(),
            datum: Datum::CH1903,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::ObliqueStereographic)
                    .with_lat0(46.952_405_556)
                    .with_lon0(7.439_583_333)
                    .with_scale(1.0)
                    .with_false_easting(600_000.0)
                    .with_false_northing(200_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2056 => Ok(Crs {
            name: "CH1903+ / LV95 (EPSG:2056)".into(),
            datum: Datum::CH1903_PLUS,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::ObliqueStereographic)
                    .with_lat0(46.952_405_556)
                    .with_lon0(7.439_583_333)
                    .with_scale(1.0)
                    .with_false_easting(2_600_000.0)
                    .with_false_northing(1_200_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        // ── Japan Plane Rectangular ──────────────────────────────────────
        2443 => japan_plane_jgd2000(code, 1,  129.5, 33.0),
        2444 => japan_plane_jgd2000(code, 2,  131.0, 33.0),
        2445 => japan_plane_jgd2000(code, 3,  132.166_667, 36.0),
        2446 => japan_plane_jgd2000(code, 4,  133.5, 33.0),
        2447 => japan_plane_jgd2000(code, 5,  134.333_333, 36.0),
        2448 => japan_plane_jgd2000(code, 6,  136.0, 36.0),
        2449 => japan_plane_jgd2000(code, 7,  137.166_667, 36.0),
        2450 => japan_plane_jgd2000(code, 8,  138.5, 36.0),
        2451 => japan_plane_jgd2000(code, 9,  139.833_333, 36.0),
        2452 => japan_plane_jgd2000(code, 10, 140.833_333, 40.0),
        2453 => japan_plane_jgd2000(code, 11, 140.25, 44.0),
        2454 => japan_plane_jgd2000(code, 12, 142.25, 44.0),
        2455 => japan_plane_jgd2000(code, 13, 144.25, 44.0),
        2456 => japan_plane_jgd2000(code, 14, 142.0, 26.0),
        2457 => japan_plane_jgd2000(code, 15, 127.5, 26.0),
        2458 => japan_plane_jgd2000(code, 16, 124.0, 26.0),
        2459 => japan_plane_jgd2000(code, 17, 131.0, 26.0),
        2460 => japan_plane_jgd2000(code, 18, 136.0, 20.0),
        2461 => japan_plane_jgd2000(code, 19, 154.0, 26.0),

        6669 => japan_plane(code, 1,  130.0, 33.0),
        6670 => japan_plane(code, 2,  131.0, 33.0),
        6671 => japan_plane(code, 3,  132.166_667, 36.0),
        6672 => japan_plane(code, 4,  133.5, 36.0),
        6673 => japan_plane(code, 5,  134.333_333, 36.0),
        6674 => japan_plane(code, 6,  136.0, 36.0),
        6675 => japan_plane(code, 7,  137.166_667, 36.0),
        6676 => japan_plane(code, 8,  138.5, 36.0),
        6677 => japan_plane(code, 9,  139.833_333, 36.0),
        6678 => japan_plane(code, 10, 140.833_333, 40.0),
        6679 => japan_plane(code, 11, 140.25, 44.0),
        6680 => japan_plane(code, 12, 142.25, 44.0),
        6681 => japan_plane(code, 13, 144.25, 44.0),
        6682 => japan_plane(code, 14, 142.0, 26.0),
        6683 => japan_plane(code, 15, 127.5, 26.0),
        6684 => japan_plane(code, 16, 124.0, 26.0),
        6685 => japan_plane(code, 17, 131.0, 26.0),
        6686 => japan_plane(code, 18, 136.0, 20.0),
        6687 => japan_plane(code, 19, 154.0, 26.0),
        6688 => jgd2011_utm_crs(code, 51),
        6689 => jgd2011_utm_crs(code, 52),
        6690 => jgd2011_utm_crs(code, 53),
        6691 => jgd2011_utm_crs(code, 54),
        6692 => jgd2011_utm_crs(code, 55),

        6707 => rdn2008_utm_crs(code, 32),
        6708 => rdn2008_utm_crs(code, 33),
        6709 => rdn2008_utm_crs(code, 34),

        6732 => gda94_mga_variant_crs(code, 41),
        6733 => gda94_mga_variant_crs(code, 42),
        6734 => gda94_mga_variant_crs(code, 43),
        6735 => gda94_mga_variant_crs(code, 44),
        6736 => gda94_mga_variant_crs(code, 46),
        6737 => gda94_mga_variant_crs(code, 47),
        6738 => gda94_mga_variant_crs(code, 59),

        6784 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Baker zone (m)", Datum::NAD83,
                      -117.833_333_333_333, 44.5, 1.00016, 40_000.0, 0.0),
        6786 => us_state_plane_tm(code, "NAD83(2011) / Oregon Baker zone (m)", Datum::NAD83,
                      -117.833_333_333_333, 44.5, 1.00016, 40_000.0, 0.0),
        6788 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Bend-Klamath Falls zone (m)", Datum::NAD83,
                      -121.75, 41.75, 1.0002, 80_000.0, 0.0),
        6790 => us_state_plane_tm(code, "NAD83(2011) / Oregon Bend-Klamath Falls zone (m)", Datum::NAD83,
                      -121.75, 41.75, 1.0002, 80_000.0, 0.0),
        6800 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Canyonville-Grants Pass zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 42.5, 1.00007, 40_000.0, 0.0),
        6802 => us_state_plane_tm(code, "NAD83(2011) / Oregon Canyonville-Grants Pass zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 42.5, 1.00007, 40_000.0, 0.0),
        6812 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Cottage Grove-Canyonville zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 42.833_333_333_333_3, 1.000023, 50_000.0, 0.0),
        6814 => us_state_plane_tm(code, "NAD83(2011) / Oregon Cottage Grove-Canyonville zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 42.833_333_333_333_3, 1.000023, 50_000.0, 0.0),
        6816 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Dufur-Madras zone (m)", Datum::NAD83,
                      -121.0, 44.5, 1.00011, 80_000.0, 0.0),
        6818 => us_state_plane_tm(code, "NAD83(2011) / Oregon Dufur-Madras zone (m)", Datum::NAD83,
                      -121.0, 44.5, 1.00011, 80_000.0, 0.0),
        6820 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Eugene zone (m)", Datum::NAD83,
                      -123.166_666_666_667, 43.75, 1.000015, 50_000.0, 0.0),
        6822 => us_state_plane_tm(code, "NAD83(2011) / Oregon Eugene zone (m)", Datum::NAD83,
                      -123.166_666_666_667, 43.75, 1.000015, 50_000.0, 0.0),
        6824 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Grants Pass-Ashland zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 41.75, 1.000043, 50_000.0, 0.0),
        6826 => us_state_plane_tm(code, "NAD83(2011) / Oregon Grants Pass-Ashland zone (m)", Datum::NAD83,
                      -123.333_333_333_333, 41.75, 1.000043, 50_000.0, 0.0),
        6828 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Gresham-Warm Springs zone (m)", Datum::NAD83,
                      -122.333_333_333_333, 45.0, 1.00005, 10_000.0, 0.0),
        6830 => us_state_plane_tm(code, "NAD83(2011) / Oregon Gresham-Warm Springs zone (m)", Datum::NAD83,
                      -122.333_333_333_333, 45.0, 1.00005, 10_000.0, 0.0),
        6832 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon La Grande zone (m)", Datum::NAD83,
                      -118.0, 45.0, 1.00013, 40_000.0, 0.0),
        6834 => us_state_plane_tm(code, "NAD83(2011) / Oregon La Grande zone (m)", Datum::NAD83,
                      -118.0, 45.0, 1.00013, 40_000.0, 0.0),
        6836 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Ontario zone (m)", Datum::NAD83,
                      -117.0, 43.25, 1.0001, 80_000.0, 0.0),
        6838 => us_state_plane_tm(code, "NAD83(2011) / Oregon Ontario zone (m)", Datum::NAD83,
                      -117.0, 43.25, 1.0001, 80_000.0, 0.0),
        6844 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Pendleton zone (m)", Datum::NAD83,
                      -119.166_666_666_667, 45.25, 1.000045, 60_000.0, 0.0),
        6846 => us_state_plane_tm(code, "NAD83(2011) / Oregon Pendleton zone (m)", Datum::NAD83,
                      -119.166_666_666_667, 45.25, 1.000045, 60_000.0, 0.0),
        6848 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Pendleton-La Grande zone (m)", Datum::NAD83,
                      -118.333_333_333_333, 45.083_333_333_333_3, 1.000175, 30_000.0, 0.0),
        6850 => us_state_plane_tm(code, "NAD83(2011) / Oregon Pendleton-La Grande zone (m)", Datum::NAD83,
                      -118.333_333_333_333, 45.083_333_333_333_3, 1.000175, 30_000.0, 0.0),
        6856 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Salem zone (m)", Datum::NAD83,
                      -123.083_333_333_333, 44.333_333_333_333_3, 1.00001, 50_000.0, 0.0),
        6858 => us_state_plane_tm(code, "NAD83(2011) / Oregon Salem zone (m)", Datum::NAD83,
                      -123.083_333_333_333, 44.333_333_333_333_3, 1.00001, 50_000.0, 0.0),
        6860 => us_state_plane_tm(code, "NAD83(CORS96) / Oregon Santiam Pass zone (m)", Datum::NAD83,
                      -122.5, 44.083_333_333_333_3, 1.000155, 0.0, 0.0),
        6862 => us_state_plane_tm(code, "NAD83(2011) / Oregon Santiam Pass zone (m)", Datum::NAD83,
                      -122.5, 44.083_333_333_333_3, 1.000155, 0.0, 0.0),

        6870 => Ok(Crs {
            name: "ETRS89 / Albania TM 2010 (EPSG:6870)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(20.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        6875 => Ok(Crs {
            name: "RDN2008 / Italy zone (N-E) (EPSG:6875)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(12.0)
                    .with_lat0(0.0)
                    .with_scale(0.9985)
                    .with_false_easting(7_000_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        6876 => Ok(Crs {
            name: "RDN2008 / Zone 12 (N-E) (EPSG:6876)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(12.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(3_000_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        6915 => Ok(Crs {
            name: "South East Island 1943 / UTM zone 40N (EPSG:6915)".into(),
            datum: Datum::SOUTH_EAST_ISLAND_1943,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(57.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        6927 => Ok(Crs {
            name: "SVY21 / Singapore TM (EPSG:6927)".into(),
            datum: Datum::SVY21,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(103.833_333_333_333)
                    .with_lat0(1.366_666_666_666_67)
                    .with_scale(1.0)
                    .with_false_easting(28_001.642)
                    .with_false_northing(38_744.572)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        6956 => Ok(Crs {
            name: "VN-2000 / TM-3 zone 481 (EPSG:6956)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(102.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(0.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        6957 => Ok(Crs {
            name: "VN-2000 / TM-3 zone 482 (EPSG:6957)".into(),
            datum: Datum::VN2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(105.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(0.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        7057 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 1", Datum::NAD83,
                      -95.25, 43.2, 1.000052, 11_500_000.0, 9_600_000.0),
        7058 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 2", Datum::NAD83,
                      -92.75, 43.166_666_666_666_7, 1.000043, 12_500_000.0, 9_800_000.0),
        7059 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 3", Datum::NAD83,
                      -91.2, 40.25, 1.000035, 13_500_000.0, 8_300_000.0),
        7060 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 4", Datum::NAD83,
                      -94.833_333_333_333_3, 42.533_333_333_333_3, 1.000045, 14_500_000.0, 8_600_000.0),
        7061 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 5", Datum::NAD83,
                      -92.25, 42.65, 1.000032, 15_500_000.0, 8_900_000.0),
        7062 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 6", Datum::NAD83,
                      -95.733_333_333_333_3, 40.25, 1.000039, 16_500_000.0, 6_600_000.0),
        7063 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 7", Datum::NAD83,
                      -94.633_333_333_333_3, 40.25, 1.000045, 17_500_000.0, 6_800_000.0),
        7064 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 8", Datum::NAD83,
                      -93.716_666_666_666_7, 40.25, 1.000033, 18_500_000.0, 7_000_000.0),
        7065 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 9", Datum::NAD83,
                      -92.816_666_666_666_7, 40.25, 1.000027, 19_500_000.0, 7_200_000.0),
        7066 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 10", Datum::NAD83,
                      -91.666_666_666_666_7, 41.833_333_333_333_3, 1.00002, 20_500_000.0, 8_000_000.0),
        7067 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 11", Datum::NAD83,
                      -90.533_333_333_333_3, 40.25, 1.000027, 21_500_000.0, 7_600_000.0),
        7068 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 12", Datum::NAD83,
                      -93.75, 40.916_666_666_666_7, 1.000037, 22_500_000.0, 6_200_000.0),
        7069 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 13", Datum::NAD83,
                      -91.916_666_666_666_7, 40.25, 1.00002, 23_500_000.0, 6_400_000.0),
        7070 => us_state_plane_tm(code, "NAD83(2011) / IaRCS zone 14", Datum::NAD83,
                      -91.25, 40.25, 1.000018, 24_500_000.0, 6_200_000.0),

        7109 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS St Mary (m)", Datum::NAD83,
                      -112.5, 48.5, 1.00016, 150_000.0, 0.0),
        7110 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Blackfeet (m)", Datum::NAD83,
                      -112.5, 48.0, 1.00019, 100_000.0, 0.0),
        7111 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Milk River (m)", Datum::NAD83,
                      -111.0, 48.5, 1.000145, 150_000.0, 200_000.0),
        7112 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Fort Belknap (m)", Datum::NAD83,
                      -108.5, 48.5, 1.00012, 200_000.0, 150_000.0),
        7113 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Fort Peck Assiniboine (m)", Datum::NAD83,
                      -105.5, 48.333_333_333_333_3, 1.00012, 200_000.0, 100_000.0),
        7114 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Fort Peck Sioux (m)", Datum::NAD83,
                      -105.5, 48.333_333_333_333_3, 1.00009, 100_000.0, 50_000.0),
        7115 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Crow (m)", Datum::NAD83,
                      -107.75, 44.75, 1.000148, 200_000.0, 0.0),
        7116 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Bobcat (m)", Datum::NAD83,
                      -111.25, 46.25, 1.000185, 100_000.0, 100_000.0),
        7117 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Billings (m)", Datum::NAD83,
                      -108.416_666_666_667, 45.783_333_333_333_3, 1.0001515, 200_000.0, 50_000.0),
        7118 => us_state_plane_tm(code, "NAD83(2011) / RMTCRS Wind River (m)", Datum::NAD83,
                      -108.333_333_333_333, 42.666_666_666_666_7, 1.00024, 100_000.0, 0.0),

        7131 => us_state_plane_tm(code, "NAD83(2011) / San Francisco CS13", Datum::NAD83,
                      -122.45, 37.75, 1.000007, 48_000.0, 24_000.0),

        7257 => us_state_plane_tm(code, "NAD83(2011) / InGCS Adams (m)", Datum::NAD83,
                      -84.95, 40.55, 1.000034, 240_000.0, 36_000.0),
        7258 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Adams (ftUS)", Datum::NAD83,
                  -84.95, 40.55, 1.000034, 787_400.0, 118_110.0),
        7259 => us_state_plane_tm(code, "NAD83(2011) / InGCS Allen (m)", Datum::NAD83,
                      -85.05, 40.9, 1.000031, 240_000.0, 36_000.0),
        7260 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Allen (ftUS)", Datum::NAD83,
                  -85.05, 40.9, 1.000031, 787_400.0, 118_110.0),
        7261 => us_state_plane_tm(code, "NAD83(2011) / InGCS Bartholomew (m)", Datum::NAD83,
                      -85.85, 39.0, 1.000026, 240_000.0, 36_000.0),
        7262 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Bartholomew (ftUS)", Datum::NAD83,
                  -85.85, 39.0, 1.000026, 787_400.0, 118_110.0),
        7263 => us_state_plane_tm(code, "NAD83(2011) / InGCS Benton (m)", Datum::NAD83,
                      -87.3, 40.45, 1.000029, 240_000.0, 36_000.0),
        7264 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Benton (ftUS)", Datum::NAD83,
                  -87.3, 40.45, 1.000029, 787_400.0, 118_110.0),
        7265 => us_state_plane_tm(code, "NAD83(2011) / InGCS Blackford-Delaware (m)", Datum::NAD83,
                      -85.4, 40.05, 1.000038, 240_000.0, 36_000.0),
        7266 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Blackford-Delaware (ftUS)", Datum::NAD83,
                  -85.4, 40.05, 1.000038, 787_400.0, 118_110.0),
        7267 => us_state_plane_tm(code, "NAD83(2011) / InGCS Boone-Hendricks (m)", Datum::NAD83,
                      -86.5, 39.6, 1.000036, 240_000.0, 36_000.0),
        7268 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Boone-Hendricks (ftUS)", Datum::NAD83,
                  -86.5, 39.6, 1.000036, 787_400.0, 118_110.0),
        7269 => us_state_plane_tm(code, "NAD83(2011) / InGCS Brown (m)", Datum::NAD83,
                      -86.3, 39.0, 1.00003, 240_000.0, 36_000.0),
        7270 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Brown (ftUS)", Datum::NAD83,
                  -86.3, 39.0, 1.00003, 787_400.0, 118_110.0),
        7271 => us_state_plane_tm(code, "NAD83(2011) / InGCS Carroll (m)", Datum::NAD83,
                      -86.65, 40.4, 1.000026, 240_000.0, 36_000.0),
        7272 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Carroll (ftUS)", Datum::NAD83,
                  -86.65, 40.4, 1.000026, 787_400.0, 118_110.0),
        7273 => us_state_plane_tm(code, "NAD83(2011) / InGCS Cass (m)", Datum::NAD83,
                      -86.4, 40.55, 1.000028, 240_000.0, 36_000.0),
        7274 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Cass (ftUS)", Datum::NAD83,
                  -86.4, 40.55, 1.000028, 787_400.0, 118_110.0),
        7275 => us_state_plane_tm(code, "NAD83(2011) / InGCS Clark-Floyd-Scott (m)", Datum::NAD83,
                      -85.6, 38.15, 1.000021, 240_000.0, 36_000.0),
        7276 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Clark-Floyd-Scott (ftUS)", Datum::NAD83,
                  -85.6, 38.15, 1.000021, 787_400.0, 118_110.0),
        7277 => us_state_plane_tm(code, "NAD83(2011) / InGCS Clay (m)", Datum::NAD83,
                      -87.15, 39.15, 1.000024, 240_000.0, 36_000.0),
        7278 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Clay (ftUS)", Datum::NAD83,
                  -87.15, 39.15, 1.000024, 787_400.0, 118_110.0),
        7279 => us_state_plane_tm(code, "NAD83(2011) / InGCS Clinton (m)", Datum::NAD83,
                      -86.6, 40.15, 1.000032, 240_000.0, 36_000.0),
        7280 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Clinton (ftUS)", Datum::NAD83,
                  -86.6, 40.15, 1.000032, 787_400.0, 118_110.0),
        7281 => us_state_plane_tm(code, "NAD83(2011) / InGCS Crawford-Lawrence-Orange (m)", Datum::NAD83,
                      -86.5, 38.1, 1.000025, 240_000.0, 36_000.0),
        7282 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Crawford-Lawrence-Orange (ftUS)", Datum::NAD83,
                  -86.5, 38.1, 1.000025, 787_400.0, 118_110.0),
        7283 => us_state_plane_tm(code, "NAD83(2011) / InGCS Daviess-Greene (m)", Datum::NAD83,
                      -87.1, 38.45, 1.000018, 240_000.0, 36_000.0),
        7284 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Daviess-Greene (ftUS)", Datum::NAD83,
                  -87.1, 38.45, 1.000018, 787_400.0, 118_110.0),
        7285 => us_state_plane_tm(code, "NAD83(2011) / InGCS Dearborn-Ohio-Switzerland (m)", Datum::NAD83,
                      -84.9, 38.65, 1.000029, 240_000.0, 36_000.0),
        7286 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Dearborn-Ohio-Switzerland (ftUS)", Datum::NAD83,
                  -84.9, 38.65, 1.000029, 787_400.0, 118_110.0),
        7287 => us_state_plane_tm(code, "NAD83(2011) / InGCS Decatur-Rush (m)", Datum::NAD83,
                      -85.65, 39.1, 1.000036, 240_000.0, 36_000.0),
        7288 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Decatur-Rush (ftUS)", Datum::NAD83,
                  -85.65, 39.1, 1.000036, 787_400.0, 118_110.0),
        7289 => us_state_plane_tm(code, "NAD83(2011) / InGCS DeKalb (m)", Datum::NAD83,
                      -84.95, 41.25, 1.000036, 240_000.0, 36_000.0),
        7290 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS DeKalb (ftUS)", Datum::NAD83,
                  -84.95, 41.25, 1.000036, 787_400.0, 118_110.0),
        7291 => us_state_plane_tm(code, "NAD83(2011) / InGCS Dubois-Martin (m)", Datum::NAD83,
                      -86.95, 38.2, 1.00002, 240_000.0, 36_000.0),
        7292 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Dubois-Martin (ftUS)", Datum::NAD83,
                  -86.95, 38.2, 1.00002, 787_400.0, 118_110.0),
        7293 => us_state_plane_tm(code, "NAD83(2011) / InGCS Elkhart-Kosciusko-Wabash (m)", Datum::NAD83,
                      -85.85, 40.65, 1.000033, 240_000.0, 36_000.0),
        7294 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Elkhart-Kosciusko-Wabash (ftUS)", Datum::NAD83,
                  -85.85, 40.65, 1.000033, 787_400.0, 118_110.0),
        7295 => us_state_plane_tm(code, "NAD83(2011) / InGCS Fayette-Franklin-Union (m)", Datum::NAD83,
                      -85.05, 39.25, 1.000038, 240_000.0, 36_000.0),
        7296 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Fayette-Franklin-Union (ftUS)", Datum::NAD83,
                  -85.05, 39.25, 1.000038, 787_400.0, 118_110.0),
        7297 => us_state_plane_tm(code, "NAD83(2011) / InGCS Fountain-Warren (m)", Datum::NAD83,
                      -87.3, 39.95, 1.000025, 240_000.0, 36_000.0),
        7298 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Fountain-Warren (ftUS)", Datum::NAD83,
                  -87.3, 39.95, 1.000025, 787_400.0, 118_110.0),
        7299 => us_state_plane_tm(code, "NAD83(2011) / InGCS Fulton-Marshall-St. Joseph (m)", Datum::NAD83,
                      -86.3, 40.9, 1.000031, 240_000.0, 36_000.0),
        7300 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Fulton-Marshall-St. Joseph (ftUS)", Datum::NAD83,
                  -86.3, 40.9, 1.000031, 787_400.0, 118_110.0),
        7301 => us_state_plane_tm(code, "NAD83(2011) / InGCS Gibson (m)", Datum::NAD83,
                      -87.65, 38.15, 1.000013, 240_000.0, 36_000.0),
        7302 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Gibson (ftUS)", Datum::NAD83,
                  -87.65, 38.15, 1.000013, 787_400.0, 118_110.0),
        7303 => us_state_plane_tm(code, "NAD83(2011) / InGCS Grant (m)", Datum::NAD83,
                      -85.7, 40.35, 1.000034, 240_000.0, 36_000.0),
        7304 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Grant (ftUS)", Datum::NAD83,
                  -85.7, 40.35, 1.000034, 787_400.0, 118_110.0),
        7305 => us_state_plane_tm(code, "NAD83(2011) / InGCS Hamilton-Tipton (m)", Datum::NAD83,
                      -86.0, 39.9, 1.000034, 240_000.0, 36_000.0),
        7306 => us_state_plane_tm_ftus(code, "NAD83(2011) / InGCS Hamilton-Tipton (ftUS)", Datum::NAD83,
                  -86.0, 39.9, 1.000034, 787_400.0, 118_110.0),
        7307 => us_state_plane_tm(code, "NAD83(2011) / InGCS Hancock-Madison (m)", Datum::NAD83,
                  -85.8, 39.65, 1.000036, 240_000.0, 36_000.0),
        7309 => us_state_plane_tm(code, "NAD83(2011) / InGCS Harrison-Washington (m)", Datum::NAD83,
                  -86.15, 37.95, 1.000027, 240_000.0, 36_000.0),
        7311 => us_state_plane_tm(code, "NAD83(2011) / InGCS Henry (m)", Datum::NAD83,
                  -85.45, 39.75, 1.000043, 240_000.0, 36_000.0),
        7313 => us_state_plane_tm(code, "NAD83(2011) / InGCS Howard-Miami (m)", Datum::NAD83,
                  -86.15, 40.35, 1.000031, 240_000.0, 36_000.0),
        7315 => us_state_plane_tm(code, "NAD83(2011) / InGCS Huntington-Whitley (m)", Datum::NAD83,
                  -85.5, 40.65, 1.000034, 240_000.0, 36_000.0),
        7317 => us_state_plane_tm(code, "NAD83(2011) / InGCS Jackson (m)", Datum::NAD83,
                  -85.95, 38.7, 1.000022, 240_000.0, 36_000.0),
        7319 => us_state_plane_tm(code, "NAD83(2011) / InGCS Jasper-Porter (m)", Datum::NAD83,
                  -87.1, 40.7, 1.000027, 240_000.0, 36_000.0),
        7321 => us_state_plane_tm(code, "NAD83(2011) / InGCS Jay (m)", Datum::NAD83,
                  -85.0, 40.3, 1.000038, 240_000.0, 36_000.0),
        7323 => us_state_plane_tm(code, "NAD83(2011) / InGCS Jefferson (m)", Datum::NAD83,
                  -85.35, 38.55, 1.000028, 240_000.0, 36_000.0),
        7325 => us_state_plane_tm(code, "NAD83(2011) / InGCS Jennings (m)", Datum::NAD83,
                  -85.8, 38.8, 1.000025, 240_000.0, 36_000.0),
        7327 => us_state_plane_tm(code, "NAD83(2011) / InGCS Johnson-Marion (m)", Datum::NAD83,
                  -86.15, 39.3, 1.000031, 240_000.0, 36_000.0),
        7329 => us_state_plane_tm(code, "NAD83(2011) / InGCS Knox (m)", Datum::NAD83,
                  -87.45, 38.4, 1.000015, 240_000.0, 36_000.0),
        7331 => us_state_plane_tm(code, "NAD83(2011) / InGCS LaGrange-Noble (m)", Datum::NAD83,
                  -85.45, 41.25, 1.000037, 240_000.0, 36_000.0),
        7333 => us_state_plane_tm(code, "NAD83(2011) / InGCS Lake-Newton (m)", Datum::NAD83,
                  -87.4, 40.7, 1.000026, 240_000.0, 36_000.0),
        7335 => us_state_plane_tm(code, "NAD83(2011) / InGCS LaPorte-Pulaski-Starke (m)", Datum::NAD83,
                  -86.75, 40.9, 1.000027, 240_000.0, 36_000.0),
        7337 => us_state_plane_tm(code, "NAD83(2011) / InGCS Monroe-Morgan (m)", Datum::NAD83,
                  -86.5, 38.95, 1.000028, 240_000.0, 36_000.0),
        7339 => us_state_plane_tm(code, "NAD83(2011) / InGCS Montgomery-Putnam (m)", Datum::NAD83,
                  -86.95, 39.45, 1.000031, 240_000.0, 36_000.0),
        7341 => us_state_plane_tm(code, "NAD83(2011) / InGCS Owen (m)", Datum::NAD83,
                  -86.9, 39.15, 1.000026, 240_000.0, 36_000.0),
        7343 => us_state_plane_tm(code, "NAD83(2011) / InGCS Parke-Vermillion (m)", Datum::NAD83,
                  -87.35, 39.6, 1.000022, 240_000.0, 36_000.0),
        7345 => us_state_plane_tm(code, "NAD83(2011) / InGCS Perry (m)", Datum::NAD83,
                  -86.7, 37.8, 1.00002, 240_000.0, 36_000.0),
        7347 => us_state_plane_tm(code, "NAD83(2011) / InGCS Pike-Warrick (m)", Datum::NAD83,
                  -87.3, 37.85, 1.000015, 240_000.0, 36_000.0),
        7349 => us_state_plane_tm(code, "NAD83(2011) / InGCS Posey (m)", Datum::NAD83,
                  -87.95, 37.75, 1.000013, 240_000.0, 36_000.0),
        7351 => us_state_plane_tm(code, "NAD83(2011) / InGCS Randolph-Wayne (m)", Datum::NAD83,
                  -85.05, 39.7, 1.000044, 240_000.0, 36_000.0),
        7353 => us_state_plane_tm(code, "NAD83(2011) / InGCS Ripley (m)", Datum::NAD83,
                  -85.3, 38.9, 1.000038, 240_000.0, 36_000.0),
        7355 => us_state_plane_tm(code, "NAD83(2011) / InGCS Shelby (m)", Datum::NAD83,
                  -85.9, 39.3, 1.00003, 240_000.0, 36_000.0),

        // ── South Africa ─────────────────────────────────────────────────
        22275 => south_africa_lo(code, 15.0),
        22277 => south_africa_lo(code, 17.0),
        22279 => south_africa_lo(code, 19.0),
        22281 => south_africa_lo(code, 21.0),
        22283 => south_africa_lo(code, 23.0),
        22285 => south_africa_lo(code, 25.0),
        22287 => south_africa_lo(code, 27.0),
        22289 => south_africa_lo(code, 29.0),
        22291 => south_africa_lo(code, 31.0),
        22293 => south_africa_lo(code, 33.0),

        // ── New geographic 2D ────────────────────────────────────────────
        4283 => geographic_crs("GDA94 (EPSG:4283)", Datum::GDA94),
        4148 => geographic_crs("Hartebeesthoek94 (EPSG:4148)", Datum::WGS84),
        4152 => geographic_crs("NAD83(HARN) (EPSG:4152)", Datum::NAD83),
        4167 => geographic_crs("NZGD2000 (EPSG:4167)", Datum::NZGD2000),
        4189 => geographic_crs("RGAF09 (EPSG:4189)", Datum::WGS84),
        4619 => geographic_crs("SIRGAS95 (EPSG:4619)", Datum::SIRGAS2000),
        4681 => geographic_crs("REGVEN (EPSG:4681)", Datum::WGS84),
        4483 => geographic_crs("Mexico ITRF92 (EPSG:4483)", Datum::WGS84),
        4624 => geographic_crs("RGFG95 (EPSG:4624)", Datum::WGS84),
        4284 => geographic_crs(
            "Pulkovo 1942 (EPSG:4284)",
            Datum { name: "Pulkovo 1942", ellipsoid: Ellipsoid::KRASSOWSKY1940, transform: DatumTransform::None },
        ),
        4322 => geographic_crs(
            "WGS 72 (EPSG:4322)",
            Datum { name: "WGS 72", ellipsoid: Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26), transform: DatumTransform::None },
        ),
        6318 => geographic_crs("NAD83(2011) (EPSG:6318)", Datum::NAD83),
        4615 => geographic_crs("REGCAN95 (EPSG:4615)", Datum::WGS84),

        // ── SWEREF99 local TM zones ──────────────────────────────────────
        3007 => sweref99_local_tm(code, 12.0),
        3008 => sweref99_local_tm(code, 13.5),
        3009 => sweref99_local_tm(code, 15.0),
        3010 => sweref99_local_tm(code, 16.5),
        3011 => sweref99_local_tm(code, 18.0),
        3012 => sweref99_local_tm(code, 14.25),
        3013 => sweref99_local_tm(code, 15.75),
        3014 => sweref99_local_tm(code, 17.25),

        // ── Poland CS2000 (zones 5–8) and CS92 ──────────────────────────
        2176 => poland_cs2000(code, 15.0, 5_500_000.0),
        2177 => poland_cs2000(code, 18.0, 6_500_000.0),
        2178 => poland_cs2000(code, 21.0, 7_500_000.0),
        2179 => poland_cs2000(code, 24.0, 8_500_000.0),
        2180 => Ok(Crs {
            name: "ETRS89 / Poland CS92 (EPSG:2180)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(19.0)
                    .with_lat0(0.0)
                    .with_scale(0.9993)
                    .with_false_easting(500_000.0)
                    .with_false_northing(-5_300_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── GGRS87 / Greek Grid ──────────────────────────────────────────
        2100 => Ok(Crs {
            name: "GGRS87 / Greek Grid (EPSG:2100)".into(),
            datum: Datum { name: "GGRS87", ellipsoid: Ellipsoid::GRS80, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(24.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── HD72 / EOV (Hungary) ─────────────────────────────────────────
        23700 => {
            let ell = Ellipsoid::from_a_inv_f("GRS 1967", 6_378_160.0, 298.247_167_427);
            let datum = Datum { name: "HD72", ellipsoid: ell.clone(), transform: DatumTransform::None };
            Ok(Crs {
                name: "HD72 / EOV (EPSG:23700)".into(),
                datum,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::HotineObliqueMercator {
                        azimuth: 90.0,
                        rectified_grid_angle: None,
                    })
                        .with_lat0(47.144_393_722_222)
                        .with_lon0(19.048_571_777_778)
                        .with_scale(0.99993)
                        .with_false_easting(650_000.0)
                        .with_false_northing(200_000.0)
                        .with_ellipsoid(ell),
                )?,
            })
        }

        // ── Dealul Piscului 1970 / Stereo 70 (Romania) ──────────────────
        31700 => {
            let datum = Datum { name: "Dealul Piscului 1970", ellipsoid: Ellipsoid::KRASSOWSKY1940, transform: DatumTransform::None };
            Ok(Crs {
                name: "Dealul Piscului 1970 / Stereo 70 (EPSG:31700)".into(),
                datum,
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::Stereographic)
                        .with_lat0(46.0)
                        .with_lon0(25.0)
                        .with_scale(0.99975)
                        .with_false_easting(500_000.0)
                        .with_false_northing(500_000.0)
                        .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
                )?,
            })
        }

        // ── ETRS89 / Portugal TM06 ───────────────────────────────────────
        3763 => Ok(Crs {
            name: "ETRS89 / Portugal TM06 (EPSG:3763)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-8.133_108_333)
                    .with_lat0(39.668_258)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── HTRS96 / Croatia TM ──────────────────────────────────────────
        3765 => Ok(Crs {
            name: "HTRS96 / Croatia TM (EPSG:3765)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(16.5)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── ETRS89 / Estonian CRS L-EST97 ────────────────────────────────
        3301 => Ok(Crs {
            name: "ETRS89 / Estonian CRS L-EST97 (EPSG:3301)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 59.333_333,
                    lat2: Some(58.166_666_667),
                })
                .with_lon0(24.0)
                .with_lat0(57.517_553_778)
                .with_false_easting(500_000.0)
                .with_false_northing(6_375_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── ETRS89 / LCC Germany (N) ─────────────────────────────────────
        5243 => Ok(Crs {
            name: "ETRS89 / LCC Germany (N) (EPSG:5243)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 48.666_666_667,
                    lat2: Some(53.666_666_667),
                })
                .with_lon0(10.0)
                .with_lat0(51.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── Israel 1993 / Israeli TM Grid ────────────────────────────────
        2039 => Ok(Crs {
            name: "Israel 1993 / Israeli TM Grid (EPSG:2039)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(35.204_516_944)
                    .with_lat0(31.734_393_611)
                    .with_scale(1.000_006_7)
                    .with_false_easting(219_529.584)
                    .with_false_northing(626_907.39)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── SVY21 / Singapore TM ─────────────────────────────────────────
        3414 => Ok(Crs {
            name: "SVY21 / Singapore TM (EPSG:3414)".into(),
            datum: Datum::SVY21,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(103.833_333_333)
                    .with_lat0(1.366_666_667)
                    .with_scale(1.0)
                    .with_false_easting(28_001.642)
                    .with_false_northing(38_744.572)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        // ── Hong Kong 1980 Grid ──────────────────────────────────────────
        2326 => Ok(Crs {
            name: "Hong Kong 1980 Grid (EPSG:2326)".into(),
            datum: Datum { name: "Hong Kong 1980", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(114.178_554_958)
                    .with_lat0(22.312_133_333)
                    .with_scale(1.0)
                    .with_false_easting(836_694.05)
                    .with_false_northing(819_069.8)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        // ── NAD83 / Statistics Canada Lambert ────────────────────────────
        3347 => Ok(Crs {
            name: "NAD83 / Statistics Canada Lambert (EPSG:3347)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 49.0,
                    lat2: Some(77.0),
                })
                .with_lon0(-91.866_666_667)
                .with_lat0(63.390_675)
                .with_false_easting(6_200_000.0)
                .with_false_northing(3_000_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── NAD83 / Canada Atlas Lambert ─────────────────────────────────
        3978 => Ok(Crs {
            name: "NAD83 / Canada Atlas Lambert (EPSG:3978)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 49.0,
                    lat2: Some(77.0),
                })
                .with_lon0(-95.0)
                .with_lat0(49.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── NAD83 / Great Lakes and St Lawrence Albers ───────────────────
        3174 => Ok(Crs {
            name: "NAD83 / Great Lakes and St Lawrence Albers (EPSG:3174)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 42.122_774,
                    lat2: 49.012_27,
                })
                .with_lon0(-84.455_955)
                .with_lat0(45.568_977)
                .with_false_easting(1_000_000.0)
                .with_false_northing(1_000_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── NAD83(2011) / CONUS Albers ────────────────────────────────────
        6350 => Ok(Crs {
            name: "NAD83(2011) / CONUS Albers (EPSG:6350)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 29.5,
                    lat2: 45.5,
                })
                .with_lon0(-96.0)
                .with_lat0(23.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── GDA94 / VicGrid (Victoria, Australia) ────────────────────────
        3111 => Ok(Crs {
            name: "GDA94 / VicGrid (EPSG:3111)".into(),
            datum: Datum::GDA94,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: -36.0,
                    lat2: Some(-38.0),
                })
                .with_lon0(145.0)
                .with_lat0(-37.0)
                .with_false_easting(2_500_000.0)
                .with_false_northing(2_500_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── GDA94 / NSW Lambert (New South Wales, Australia) ─────────────
        3308 => Ok(Crs {
            name: "GDA94 / NSW Lambert (EPSG:3308)".into(),
            datum: Datum::GDA94,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: -30.75,
                    lat2: Some(-35.75),
                })
                .with_lon0(147.0)
                .with_lat0(-33.5)
                .with_false_easting(9_300_000.0)
                .with_false_northing(4_500_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        // ── Additional EPSG batch ────────────────────────────────────────
        3005 => Ok(Crs {
            name: "NAD83 / BC Albers (EPSG:3005)".into(),
            datum: Datum::NAD83,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
                    lat1: 50.0,
                    lat2: 58.5,
                })
                .with_lon0(-126.0)
                .with_lat0(45.0)
                .with_false_easting(1_000_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3015 => sweref99_local_tm(code, 18.75),

        3112 => Ok(Crs {
            name: "GDA94 / Geoscience Australia Lambert (EPSG:3112)".into(),
            datum: Datum::GDA94,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: -18.0,
                    lat2: Some(-36.0),
                })
                .with_lon0(134.0)
                .with_lat0(0.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3767 => Ok(Crs {
            name: "HTRS96 / UTM zone 33N (EPSG:3767)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(33, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2191 => Ok(Crs {
            name: "Madeira 1936 / UTM zone 28N (EPSG:2191)".into(),
            datum: Datum {
                name: "Madeira 1936",
                ellipsoid: Ellipsoid::INTERNATIONAL,
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(28, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2192 => Ok(Crs {
            name: "ED50 / France EuroLambert (EPSG:2192)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 46.8,
                    lat2: Some(46.8),
                })
                .with_lon0(2.337_229_166_666_667)
                .with_lat0(46.8)
                .with_scale(0.999_877_42)
                .with_false_easting(600_000.0)
                .with_false_northing(2_200_000.0)
                .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),


        3812 => Ok(Crs {
            name: "ETRS89 / Belgian Lambert 2008 (EPSG:3812)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 49.833_333_333_333_3,
                    lat2: Some(51.166_666_666_666_7),
                })
                .with_lon0(4.359_215_833_333_33)
                .with_lat0(50.797_815)
                .with_false_easting(649_328.0)
                .with_false_northing(665_262.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3825 => Ok(Crs {
            name: "TWD97 / TM2 zone 119 (EPSG:3825)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(119.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(250_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        3826 => Ok(Crs {
            name: "TWD97 / TM2 zone 121 (EPSG:3826)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(121.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(250_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5179 => Ok(Crs {
            name: "KGD2002 / Unified CS (EPSG:5179)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(127.5)
                    .with_lat0(38.0)
                    .with_scale(0.9996)
                    .with_false_easting(1_000_000.0)
                    .with_false_northing(2_000_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5181 => Ok(Crs {
            name: "KGD2002 / Central Belt (EPSG:5181)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(127.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5182 => Ok(Crs {
            name: "KGD2002 / Central Belt Jeju (EPSG:5182)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(127.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(550_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5186 => Ok(Crs {
            name: "KGD2002 / Central Belt 2010 (EPSG:5186)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(127.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(600_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5187 => Ok(Crs {
            name: "KGD2002 / East Belt 2010 (EPSG:5187)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(129.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(600_000.0)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        31256 => Ok(Crs {
            name: "MGI / Austria GK East (EPSG:31256)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(16.333_333_333_333_3)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(-5_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31257 => Ok(Crs {
            name: "MGI / Austria GK M28 (EPSG:31257)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(10.333_333_333_333_3)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(150_000.0)
                    .with_false_northing(-5_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31258 => Ok(Crs {
            name: "MGI / Austria GK M31 (EPSG:31258)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(13.333_333_333_333_3)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(450_000.0)
                    .with_false_northing(-5_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31287 => Ok(Crs {
            name: "MGI / Austria Lambert (EPSG:31287)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 49.0,
                    lat2: Some(46.0),
                })
                .with_lon0(13.333_333_333_333_3)
                .with_lat0(47.5)
                .with_false_easting(400_000.0)
                .with_false_northing(400_000.0)
                .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2046 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo15 (EPSG:2046)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2047 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo17 (EPSG:2047)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(17.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        // ── Additional EPSG batch II ───────────────────────────────────
        7845 => Ok(Crs {
            name: "GDA2020 / GA LCC (EPSG:7845)".into(),
            datum: Datum::GDA2020,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: -18.0,
                    lat2: Some(-36.0),
                })
                .with_lon0(134.0)
                .with_lat0(0.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        5513 => Ok(Crs {
            name: "S-JTSK / Krovak (EPSG:5513)".into(),
            datum: Datum::S_JTSK,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Krovak)
                    .with_lat0(49.5)
                    .with_lon0(7.166_666_666_666_667)
                    .with_scale(0.9999)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2065 => Ok(Crs {
            name: "S-JTSK (Ferro) / Krovak (EPSG:2065)".into(),
            datum: Datum { name: "S-JTSK (Ferro)", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::Krovak)
                    .with_lat0(49.5)
                    // 42.5° with Ferro prime meridian equals 24°50' Greenwich.
                    .with_lon0(24.833_333_333_333_3)
                    .with_scale(0.9999)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31254 => Ok(Crs {
            name: "MGI / Austria GK West (EPSG:31254)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(10.333_333_333_333_3)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(-5_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31255 => Ok(Crs {
            name: "MGI / Austria GK Central (EPSG:31255)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(13.333_333_333_333_3)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(-5_000_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31265 => Ok(Crs {
            name: "MGI / 3-degree Gauss zone 5 (EPSG:31265)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(5_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31266 => Ok(Crs {
            name: "MGI / 3-degree Gauss zone 6 (EPSG:31266)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(18.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(6_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31267 => Ok(Crs {
            name: "MGI / 3-degree Gauss zone 7 (EPSG:31267)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(21.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(7_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        3766 => Ok(Crs {
            name: "HTRS96 / Croatia LCC (EPSG:3766)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 45.916_666_666_666_7,
                    lat2: Some(43.083_333_333_333_3),
                })
                .with_lon0(16.5)
                .with_lat0(0.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2048 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo19 (EPSG:2048)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(19.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2049 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo21 (EPSG:2049)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(21.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2050 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo23 (EPSG:2050)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(23.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2051 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo25 (EPSG:2051)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(25.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2052 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo27 (EPSG:2052)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(27.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2053 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo29 (EPSG:2053)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(29.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2054 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo31 (EPSG:2054)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(31.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2055 => Ok(Crs {
            name: "Hartebeesthoek94 / Lo33 (EPSG:2055)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(33.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(0.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2040 => Ok(Crs {
            name: "Locodjo 1965 / UTM zone 30N (EPSG:2040)".into(),
            datum: Datum { name: "Locodjo 1965", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(30, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2041 => Ok(Crs {
            name: "Abidjan 1987 / UTM zone 30N (EPSG:2041)".into(),
            datum: Datum { name: "Abidjan 1987", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(30, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2042 => Ok(Crs {
            name: "Locodjo 1965 / UTM zone 29N (EPSG:2042)".into(),
            datum: Datum { name: "Locodjo 1965", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(29, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2043 => Ok(Crs {
            name: "Abidjan 1987 / UTM zone 29N (EPSG:2043)".into(),
            datum: Datum { name: "Abidjan 1987", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(29, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2057 => Ok(Crs {
            name: "Rassadiran / Nakhl-e Taqi (EPSG:2057)".into(),
            datum: Datum { name: "Rassadiran", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::HotineObliqueMercator {
                    azimuth: 0.571_661_194_444_444_4,
                    rectified_grid_angle: None,
                })
                    .with_lon0(52.603_539_166_666_67)
                    .with_lat0(27.518_828_805_555_55)
                    .with_scale(0.999_895_934)
                    .with_false_easting(658_377.437)
                    .with_false_northing(3_044_969.194)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2058 => Ok(Crs {
            name: "ED50(ED77) / UTM zone 38N (EPSG:2058)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(38, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2059 => Ok(Crs {
            name: "ED50(ED77) / UTM zone 39N (EPSG:2059)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(39, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2060 => Ok(Crs {
            name: "ED50(ED77) / UTM zone 40N (EPSG:2060)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(40, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2061 => Ok(Crs {
            name: "ED50(ED77) / UTM zone 41N (EPSG:2061)".into(),
            datum: Datum::ED50,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(41, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2063 => Ok(Crs {
            name: "Dabola 1981 / UTM zone 28N (EPSG:2063)".into(),
            datum: Datum {
                name: "Dabola 1981",
                ellipsoid: Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(28, false)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5)),
            )?,
        }),

        2064 => Ok(Crs {
            name: "Dabola 1981 / UTM zone 29N (EPSG:2064)".into(),
            datum: Datum {
                name: "Dabola 1981",
                ellipsoid: Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(29, false)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5)),
            )?,
        }),

        2067 => Ok(Crs {
            name: "Naparima 1955 / UTM zone 20N (EPSG:2067)".into(),
            datum: Datum { name: "Naparima 1955", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(20, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2068 => Ok(Crs {
            name: "ELD79 / Libya zone 5 (EPSG:2068)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(9.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2069 => Ok(Crs {
            name: "ELD79 / Libya zone 6 (EPSG:2069)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(11.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2070 => Ok(Crs {
            name: "ELD79 / Libya zone 7 (EPSG:2070)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(13.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2071 => Ok(Crs {
            name: "ELD79 / Libya zone 8 (EPSG:2071)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2072 => Ok(Crs {
            name: "ELD79 / Libya zone 9 (EPSG:2072)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(17.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2073 => Ok(Crs {
            name: "ELD79 / Libya zone 10 (EPSG:2073)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(19.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2074 => Ok(Crs {
            name: "ELD79 / Libya zone 11 (EPSG:2074)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(21.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2075 => Ok(Crs {
            name: "ELD79 / Libya zone 12 (EPSG:2075)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(23.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2076 => Ok(Crs {
            name: "ELD79 / Libya zone 13 (EPSG:2076)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(25.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(200_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2077 => Ok(Crs {
            name: "ELD79 / UTM zone 32N (EPSG:2077)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(32, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2078 => Ok(Crs {
            name: "ELD79 / UTM zone 33N (EPSG:2078)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(33, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2079 => Ok(Crs {
            name: "ELD79 / UTM zone 34N (EPSG:2079)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(34, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2080 => Ok(Crs {
            name: "ELD79 / UTM zone 35N (EPSG:2080)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(35, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2085 => Ok(Crs {
            name: "NAD27 / Cuba Norte (EPSG:2085)".into(),
            datum: Datum::NAD27,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 22.35,
                    lat2: None,
                })
                .with_lon0(-81.0)
                .with_lat0(22.35)
                .with_scale(0.999_936_02)
                .with_false_easting(500_000.0)
                .with_false_northing(280_296.016)
                .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        2086 => Ok(Crs {
            name: "NAD27 / Cuba Sur (EPSG:2086)".into(),
            datum: Datum::NAD27,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 20.716_666_666_666_67,
                    lat2: None,
                })
                .with_lon0(-76.833_333_333_333_33)
                .with_lat0(20.716_666_666_666_67)
                .with_scale(0.999_948_48)
                .with_false_easting(500_000.0)
                .with_false_northing(229_126.939)
                .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        2087 => Ok(Crs {
            name: "ELD79 / TM 12 NE (EPSG:2087)".into(),
            datum: Datum { name: "European Libyan Datum 1979", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(12.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2088 => Ok(Crs {
            name: "Carthage / TM 11 NE (EPSG:2088)".into(),
            datum: Datum {
                name: "Carthage",
                ellipsoid: Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(11.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("Clarke 1880 (IGN)", 6_378_249.2, 293.466_021_293_626_5)),
            )?,
        }),

        2089 => Ok(Crs {
            name: "Yemen NGN96 / UTM zone 38N (EPSG:2089)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(38, false)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2090 => Ok(Crs {
            name: "Yemen NGN96 / UTM zone 39N (EPSG:2090)".into(),
            datum: Datum::WGS84,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(39, false)
                    .with_ellipsoid(Ellipsoid::WGS84),
            )?,
        }),

        2091 => Ok(Crs {
            name: "South Yemen / GK zone 8 (EPSG:2091)".into(),
            datum: Datum { name: "South Yemen", ellipsoid: Ellipsoid::KRASSOWSKY1940, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(45.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(8_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        2092 => Ok(Crs {
            name: "South Yemen / GK zone 9 (EPSG:2092)".into(),
            datum: Datum { name: "South Yemen", ellipsoid: Ellipsoid::KRASSOWSKY1940, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(51.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(9_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        2093 => Ok(Crs {
            name: "Hanoi 1972 / GK 106 NE (EPSG:2093)".into(),
            datum: Datum { name: "Hanoi 1972", ellipsoid: Ellipsoid::KRASSOWSKY1940, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(106.0)
                    .with_lat0(0.0)
                    .with_scale(1.0)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        2094 => Ok(Crs {
            name: "WGS 72BE / TM 106 NE (EPSG:2094)".into(),
            datum: Datum { name: "WGS 72BE", ellipsoid: Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26), transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(106.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26)),
            )?,
        }),

        2095 => Ok(Crs {
            name: "Bissau / UTM zone 28N (EPSG:2095)".into(),
            datum: Datum { name: "Bissau", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(28, false)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2096 => Ok(Crs {
            name: "Korean 1985 / East Belt (EPSG:2096)".into(),
            datum: Datum { name: "Korean Datum 1985", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(129.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2097 => Ok(Crs {
            name: "Korean 1985 / Central Belt (EPSG:2097)".into(),
            datum: Datum { name: "Korean Datum 1985", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(127.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2098 => Ok(Crs {
            name: "Korean 1985 / West Belt (EPSG:2098)".into(),
            datum: Datum { name: "Korean Datum 1985", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(125.0)
                    .with_lat0(38.0)
                    .with_scale(1.0)
                    .with_false_easting(200_000.0)
                    .with_false_northing(500_000.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2105..=2132 => {
            let (name, lon0, lat0, scale) = match code {
                2105 => ("NZGD2000 / Mount Eden Circuit", 174.764_166_666_666_7, -36.879_722_222_222_22, 0.9999),
                2106 => ("NZGD2000 / Bay of Plenty Circuit", 176.466_111_111_111_1, -37.761_111_111_111_11, 1.0),
                2107 => ("NZGD2000 / Poverty Bay Circuit", 177.885_555_555_555_6, -38.624_444_444_444_44, 1.0),
                2108 => ("NZGD2000 / Hawkes Bay Circuit", 176.673_611_111_111_1, -39.650_833_333_333_33, 1.0),
                2109 => ("NZGD2000 / Taranaki Circuit", 174.227_777_777_777_8, -39.135_555_555_555_56, 1.0),
                2110 => ("NZGD2000 / Tuhirangi Circuit", 175.64, -39.512_222_222_222_22, 1.0),
                2111 => ("NZGD2000 / Wanganui Circuit", 175.488_055_555_555_5, -40.241_944_444_444_44, 1.0),
                2112 => ("NZGD2000 / Wairarapa Circuit", 175.647_222_222_222_2, -40.925_277_777_777_77, 1.0),
                2113 => ("NZGD2000 / Wellington Circuit", 174.776_388_888_888_9, -41.301_111_111_111_1, 1.0),
                2114 => ("NZGD2000 / Collingwood Circuit", 172.671_944_444_444_4, -40.714_722_222_222_23, 1.0),
                2115 => ("NZGD2000 / Nelson Circuit", 173.299_166_666_666_7, -41.274_444_444_444_44, 1.0),
                2116 => ("NZGD2000 / Karamea Circuit", 172.108_888_888_888_9, -41.289_722_222_222_22, 1.0),
                2117 => ("NZGD2000 / Buller Circuit", 171.581_111_111_111_1, -41.810_555_555_555_55, 1.0),
                2118 => ("NZGD2000 / Grey Circuit", 171.549_722_222_222_2, -42.333_611_111_111_11, 1.0),
                2119 => ("NZGD2000 / Amuri Circuit", 173.01, -42.688_888_888_888_88, 1.0),
                2120 => ("NZGD2000 / Marlborough Circuit", 173.801_944_444_444_4, -41.544_444_444_444_44, 1.0),
                2121 => ("NZGD2000 / Hokitika Circuit", 170.979_722_222_222_2, -42.886_111_111_111_11, 1.0),
                2122 => ("NZGD2000 / Okarito Circuit", 170.260_833_333_333_3, -43.11, 1.0),
                2123 => ("NZGD2000 / Jacksons Bay Circuit", 168.606_111_111_111_1, -43.977_777_777_777_78, 1.0),
                2124 => ("NZGD2000 / Mount Pleasant Circuit", 172.726_944_444_444_5, -43.590_555_555_555_56, 1.0),
                2125 => ("NZGD2000 / Gawler Circuit", 171.360_555_555_555_5, -43.748_611_111_111_11, 1.0),
                2126 => ("NZGD2000 / Timaru Circuit", 171.057_222_222_222_2, -44.401_944_444_444_45, 1.0),
                2127 => ("NZGD2000 / Lindis Peak Circuit", 169.4675, -44.735, 1.0),
                2128 => ("NZGD2000 / Mount Nicholas Circuit", 168.398_611_111_111_1, -45.132_777_777_777_78, 1.0),
                2129 => ("NZGD2000 / Mount York Circuit", 167.738_611_111_111_1, -45.563_611_111_111_11, 1.0),
                2130 => ("NZGD2000 / Observation Point Circuit", 170.628_333_333_333_3, -45.816_111_111_111_11, 1.0),
                2131 => ("NZGD2000 / North Taieri Circuit", 170.2825, -45.861_388_888_888_89, 0.99996),
                2132 => ("NZGD2000 / Bluff Circuit", 168.342_777_777_777_8, -46.6, 1.0),
                _ => unreachable!(),
            };
            nzgd2000_circuit_tm(code, name, lon0, lat0, scale)
        }

        2133 => Ok(Crs {
            name: "NZGD2000 / UTM zone 58S (EPSG:2133)".into(),
            datum: Datum::NZGD2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(58, true)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2134 => Ok(Crs {
            name: "NZGD2000 / UTM zone 59S (EPSG:2134)".into(),
            datum: Datum::NZGD2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(59, true)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2135 => Ok(Crs {
            name: "NZGD2000 / UTM zone 60S (EPSG:2135)".into(),
            datum: Datum::NZGD2000,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(60, true)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2136 => {
            let foot_gold_coast = 0.304_799_710_181_508_8;
            let war_office_inv_f = 296.0;
            Ok(Crs {
                name: "Accra / Ghana Grid (EPSG:2136)".into(),
                datum: Datum {
                    name: "Accra",
                    ellipsoid: Ellipsoid::from_a_inv_f(
                        "War Office",
                        6_378_300.0 / foot_gold_coast,
                        war_office_inv_f,
                    ),
                    transform: DatumTransform::None,
                },
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-1.0)
                        .with_lat0(4.666_666_666_666_667)
                        .with_scale(0.99975)
                        .with_false_easting(900_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(Ellipsoid::from_a_inv_f(
                            "War Office",
                            6_378_300.0 / foot_gold_coast,
                            war_office_inv_f,
                        )),
                )?,
            })
        }

        2137 => Ok(Crs {
            name: "Accra / TM 1 NW (EPSG:2137)".into(),
            datum: Datum {
                name: "Accra",
                ellipsoid: Ellipsoid::from_a_inv_f("War Office", 6_378_300.0, 296.0),
                transform: DatumTransform::None,
            },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-1.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::from_a_inv_f("War Office", 6_378_300.0, 296.0)),
            )?,
        }),

        2138 => Ok(Crs {
            name: "NAD27(CGQ77) / Quebec Lambert (EPSG:2138)".into(),
            datum: Datum::NAD27,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::LambertConformalConic {
                    lat1: 46.0,
                    lat2: Some(60.0),
                })
                .with_lon0(-68.5)
                .with_lat0(44.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::CLARKE1866),
            )?,
        }),

        2148 => csrs_utm_crs(21, code),
        2149 => csrs_utm_crs(18, code),
        2150 => csrs_utm_crs(17, code),
        2151 => csrs_utm_crs(13, code),
        2152 => csrs_utm_crs(12, code),

        2153 => csrs_utm_crs(11, code),

        2158 => Ok(Crs {
            name: "IRENET95 / UTM zone 29N (EPSG:2158)".into(),
            datum: Datum::ETRS89,
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(29, false)
                    .with_ellipsoid(Ellipsoid::GRS80),
            )?,
        }),

        2159 => {
            let foot_gold_coast = 0.304_799_710_181_508_8;
            let a_foot = 6_378_300.0 / foot_gold_coast;
            let inv_f = 296.0;
            Ok(Crs {
                name: "Sierra Leone 1924 / New Colony Grid (EPSG:2159)".into(),
                datum: Datum {
                    name: "Sierra Leone 1924",
                    ellipsoid: Ellipsoid::from_a_inv_f("War Office", a_foot, inv_f),
                    transform: DatumTransform::None,
                },
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-12.0)
                        .with_lat0(6.666_666_666_666_667)
                        .with_scale(1.0)
                        .with_false_easting(500_000.0)
                        .with_false_northing(0.0)
                        .with_ellipsoid(Ellipsoid::from_a_inv_f("War Office", a_foot, inv_f)),
                )?,
            })
        }

        2160 => {
            let foot_gold_coast = 0.304_799_710_181_508_8;
            let a_foot = 6_378_300.0 / foot_gold_coast;
            let inv_f = 296.0;
            Ok(Crs {
                name: "Sierra Leone 1924 / New War Office Grid (EPSG:2160)".into(),
                datum: Datum {
                    name: "Sierra Leone 1924",
                    ellipsoid: Ellipsoid::from_a_inv_f("War Office", a_foot, inv_f),
                    transform: DatumTransform::None,
                },
                projection: crate::projections::Projection::new(
                    ProjectionParams::new(ProjectionKind::TransverseMercator)
                        .with_lon0(-12.0)
                        .with_lat0(6.666_666_666_666_667)
                        .with_scale(1.0)
                        .with_false_easting(800_000.0)
                        .with_false_northing(600_000.0)
                        .with_ellipsoid(Ellipsoid::from_a_inv_f("War Office", a_foot, inv_f)),
                )?,
            })
        }

        2161 => Ok(Crs {
            name: "Sierra Leone 1968 / UTM zone 28N (EPSG:2161)".into(),
            datum: Datum { name: "Sierra Leone 1968", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(28, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2162 => Ok(Crs {
            name: "Sierra Leone 1968 / UTM zone 29N (EPSG:2162)".into(),
            datum: Datum { name: "Sierra Leone 1968", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::utm(29, false)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2164 => Ok(Crs {
            name: "Locodjo 1965 / TM 5 NW (EPSG:2164)".into(),
            datum: Datum { name: "Locodjo 1965", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-5.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2165 => Ok(Crs {
            name: "Abidjan 1987 / TM 5 NW (EPSG:2165)".into(),
            datum: Datum { name: "Abidjan 1987", ellipsoid: Ellipsoid::CLARKE1880_RGS, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(-5.0)
                    .with_lat0(0.0)
                    .with_scale(0.9996)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::CLARKE1880_RGS),
            )?,
        }),

        2166 => pulkovo_adj1983_gk_3deg(code, 3, 9.0),
        2167 => pulkovo_adj1983_gk_3deg(code, 4, 12.0),
        2168 => pulkovo_adj1983_gk_3deg(code, 5, 15.0),

        2169 => Ok(Crs {
            name: "Luxembourg 1930 / Gauss (EPSG:2169)".into(),
            datum: Datum { name: "Luxembourg 1930", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(6.166_666_666_666_667)
                    .with_lat0(49.833_333_333_333_34)
                    .with_scale(1.0)
                    .with_false_easting(80_000.0)
                    .with_false_northing(100_000.0)
                    .with_ellipsoid(Ellipsoid::INTERNATIONAL),
            )?,
        }),

        2170 => Ok(Crs {
            name: "MGI / Slovenia Grid (EPSG:2170)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        2172 => pulkovo_adj1958_poland_stereographic(
            2172,
            "Pulkovo 1942 Adj 1958 / Poland zone II",
            21.502_777_777_777_78,
            53.001_944_444_444_45,
            0.9998,
            4_603_000.0,
            5_806_000.0,
        ),

        2173 => pulkovo_adj1958_poland_stereographic(
            2173,
            "Pulkovo 1942 Adj 1958 / Poland zone III",
            17.008_333_333_333_33,
            53.583_333_333_333_34,
            0.9998,
            3_501_000.0,
            5_999_000.0,
        ),

        2174 => pulkovo_adj1958_poland_stereographic(
            2174,
            "Pulkovo 1942 Adj 1958 / Poland zone IV",
            16.672_222_222_222_22,
            51.670_833_333_333_33,
            0.9998,
            3_703_000.0,
            5_627_000.0,
        ),

        2175 => Ok(Crs {
            name: "Pulkovo 1942 Adj 1958 / Poland zone V (EPSG:2175)".into(),
            datum: Datum::PULKOVO1942_58,
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(18.958_333_333_333_33)
                    .with_lat0(0.0)
                    .with_scale(0.999_983)
                    .with_false_easting(237_000.0)
                    .with_false_northing(-4_700_000.0)
                    .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
            )?,
        }),

        2397 => pulkovo_adj1983_gk_3deg(code, 3, 9.0),
        2398 => pulkovo_adj1983_gk_3deg(code, 4, 12.0),
        2399 => pulkovo_adj1983_gk_3deg(code, 5, 15.0),
        3329 => pulkovo_adj1958_gk_3deg(code, 5, 15.0),
        3330 => pulkovo_adj1958_gk_3deg(code, 6, 18.0),
        3331 => pulkovo_adj1958_gk_3deg(code, 7, 21.0),
        3332 => pulkovo_adj1958_gk_3deg(code, 8, 24.0),
        3333 => pulkovo_adj1958_gk_6deg(code, 3),
        3334 => pulkovo_adj1958_gk_6deg(code, 4),
        3335 => pulkovo_adj1958_gk_6deg(code, 5),
        4417 => pulkovo_adj1983_gk_3deg(code, 7, 21.0),
        4434 => pulkovo_adj1983_gk_3deg(code, 8, 24.0),
        5631 => pulkovo_adj1958_gk_6deg(code, 2),
        5663 => pulkovo_adj1958_gk_6deg(code, 3),
        5664 => pulkovo_adj1983_gk_6deg(code, 2),
        5665 => pulkovo_adj1983_gk_6deg(code, 3),
        5670 => pulkovo_adj1958_gk_3deg(code, 3, 9.0),
        5671 => pulkovo_adj1958_gk_3deg(code, 4, 12.0),
        5672 => pulkovo_adj1958_gk_3deg(code, 5, 15.0),
        5673 => pulkovo_adj1983_gk_3deg(code, 3, 9.0),
        5674 => pulkovo_adj1983_gk_3deg(code, 4, 12.0),
        5675 => pulkovo_adj1983_gk_3deg(code, 5, 15.0),

        31275 => Ok(Crs {
            name: "MGI / Balkans zone 5 (EPSG:31275)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(15.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(5_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        31276 => Ok(Crs {
            name: "MGI / Balkans zone 6 (EPSG:31276)".into(),
            datum: Datum { name: "MGI", ellipsoid: Ellipsoid::BESSEL, transform: DatumTransform::None },
            projection: crate::projections::Projection::new(
                ProjectionParams::new(ProjectionKind::TransverseMercator)
                    .with_lon0(18.0)
                    .with_lat0(0.0)
                    .with_scale(0.9999)
                    .with_false_easting(6_500_000.0)
                    .with_false_northing(0.0)
                    .with_ellipsoid(Ellipsoid::BESSEL),
            )?,
        }),

        _ => Err(ProjectionError::UnsupportedProjection(format!(
            "EPSG:{code} is not in the built-in registry. \
             See epsg::known_epsg_codes() for supported codes, \
             or construct a ProjectionParams manually."
        ))),
    }
}

// ─── helper constructors ────────────────────────────────────────────────────

fn geographic_crs(name: &str, datum: Datum) -> Result<Crs> {
    // Geographic CRS: lon/lat degree pass-through.
    Ok(Crs {
        name: name.to_string(),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::Geographic)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

fn custom_geographic_crs(code: u32, crs_name: &'static str, datum_name: &'static str, ellipsoid: Ellipsoid) -> Result<Crs> {
    geographic_crs(
        &format!("{crs_name} (EPSG:{code})"),
        Datum {
            name: datum_name,
            ellipsoid,
            transform: DatumTransform::None,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn custom_tm_crs(
    code: u32,
    crs_name: &'static str,
    datum_name: &'static str,
    ellipsoid: Ellipsoid,
    lon0: f64,
    lat0: f64,
    scale: f64,
    false_easting: f64,
    false_northing: f64,
) -> Result<Crs> {
    Ok(Crs {
        name: format!("{crs_name} (EPSG:{code})"),
        datum: Datum {
            name: datum_name,
            ellipsoid: ellipsoid.clone(),
            transform: DatumTransform::None,
        },
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(false_easting)
                .with_false_northing(false_northing)
                .with_ellipsoid(ellipsoid),
        )?,
    })
}

fn nzgd2000_circuit_tm(code: u32, name: &'static str, lon0: f64, lat0: f64, scale: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum: Datum::NZGD2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(400_000.0)
                .with_false_northing(800_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn pulkovo_adj1983_gk_3deg(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("Pulkovo 1942(83) / 3-degree GK zone {zone} (EPSG:{code})"),
        datum: Datum::PULKOVO1942_83,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(f64::from(zone) * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn pulkovo_adj1958_poland_stereographic(
    code: u32,
    name: &'static str,
    lon0: f64,
    lat0: f64,
    scale: f64,
    false_easting: f64,
    false_northing: f64,
) -> Result<Crs> {
    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum: Datum::PULKOVO1942_58,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::Stereographic)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(false_easting)
                .with_false_northing(false_northing)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn wrap_longitude_180(mut lon: f64) -> f64 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

fn pulkovo_gk_cm(code: u32, datum_name: &'static str, lon0: f64) -> Result<Crs> {
    let datum = Datum {
        name: datum_name,
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!("{datum_name} / Gauss-Kruger CM {:.0} (EPSG:{code})", lon0),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn pulkovo_gk_zone(code: u32, datum_name: &'static str, zone: u32) -> Result<Crs> {
    let datum = Datum {
        name: datum_name,
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::None,
    };
    let lon0 = wrap_longitude_180((zone as f64) * 3.0);
    let false_easting = (zone as f64) * 1_000_000.0 + 500_000.0;

    Ok(Crs {
        name: format!("{datum_name} / 3-degree GK zone {zone} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn pulkovo_gk_6deg_zone(code: u32, datum_name: &'static str, zone: u32) -> Result<Crs> {
    let datum = Datum {
        name: datum_name,
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::None,
    };
    let lon0 = wrap_longitude_180((zone as f64) * 6.0 - 3.0);
    let false_easting = (zone as f64) * 1_000_000.0 + 500_000.0;

    Ok(Crs {
        name: format!("{datum_name} / Gauss-Kruger zone {zone} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn pulkovo_adj1958_gk_3deg(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    let false_easting = (zone as f64) * 1_000_000.0 + 500_000.0;
    Ok(Crs {
        name: format!("Pulkovo 1942 Adj 1958 / 3-degree GK zone {zone} (EPSG:{code})"),
        datum: Datum::PULKOVO1942_58,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn pulkovo_adj1958_gk_6deg(code: u32, zone: u32) -> Result<Crs> {
    let lon0 = wrap_longitude_180((zone as f64) * 6.0 - 3.0);
    let false_easting = (zone as f64) * 1_000_000.0 + 500_000.0;
    Ok(Crs {
        name: format!("Pulkovo 1942 Adj 1958 / GK zone {zone} (EPSG:{code})"),
        datum: Datum::PULKOVO1942_58,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn pulkovo_adj1983_gk_6deg(code: u32, zone: u32) -> Result<Crs> {
    let lon0 = wrap_longitude_180((zone as f64) * 6.0 - 3.0);
    let false_easting = (zone as f64) * 1_000_000.0 + 500_000.0;
    Ok(Crs {
        name: format!("Pulkovo 1942 Adj 1983 / GK zone {zone} (EPSG:{code})"),
        datum: Datum::PULKOVO1942_83,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn wgs72_utm_crs(code: u32, zone: u8, south: bool) -> Result<Crs> {
    let ellipsoid = Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26);
    let datum = Datum {
        name: "WGS 72",
        ellipsoid: ellipsoid.clone(),
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!(
            "WGS 72 / UTM zone {}{} (EPSG:{code})",
            zone,
            if south { "S" } else { "N" }
        ),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, south)
                .with_ellipsoid(ellipsoid),
        )?,
    })
}

fn wgs72be_utm_crs(code: u32, zone: u8, south: bool) -> Result<Crs> {
    let ellipsoid = Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26);
    let datum = Datum {
        name: "WGS 72BE",
        ellipsoid: ellipsoid.clone(),
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!(
            "WGS 72BE / UTM zone {}{} (EPSG:{code})",
            zone,
            if south { "S" } else { "N" }
        ),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, south)
                .with_ellipsoid(ellipsoid),
        )?,
    })
}

fn csrs_utm_crs(zone: u8, code: u32) -> Result<Crs> {
    csrs_utm_crs_variant(zone, code, "")
}

fn csrs_utm_crs_variant(zone: u8, code: u32, version_suffix: &str) -> Result<Crs> {
    let realization = if version_suffix.is_empty() {
        "NAD83(CSRS)".to_string()
    } else {
        format!("NAD83(CSRS){version_suffix}")
    };

    Ok(Crs {
        name: format!("{} / UTM zone {}N (EPSG:{code})", realization, zone),
        datum: Datum::NAD83_CSRS,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, false)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn sirgas2000_utm_crs(code: u32, zone: u8, south: bool) -> Result<Crs> {
    Ok(Crs {
        name: format!(
            "SIRGAS 2000 / UTM zone {}{} (EPSG:{code})",
            zone,
            if south { "S" } else { "N" }
        ),
        datum: Datum::SIRGAS2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, south)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn nad83_2011_utm_crs(code: u32, zone: u8) -> Result<Crs> {
    let datum = Datum {
        name: "NAD83(2011)",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!("NAD83(2011) / UTM zone {}N (EPSG:{code})", zone),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, false)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

fn sad69_utm_crs(code: u32, zone: u8, south: bool) -> Result<Crs> {
    let datum = Datum {
        name: "SAD69",
        ellipsoid: Ellipsoid::from_a_inv_f("GRS 1967 Modified", 6_378_160.0, 298.25),
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!(
            "SAD69 / UTM zone {}{} (EPSG:{code})",
            zone,
            if south { "S" } else { "N" }
        ),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, south)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

fn psad56_utm_crs(code: u32, zone: u8, south: bool) -> Result<Crs> {
    let datum = Datum {
        name: "PSAD56",
        ellipsoid: Ellipsoid::INTERNATIONAL,
        transform: DatumTransform::None,
    };

    Ok(Crs {
        name: format!(
            "PSAD56 / UTM zone {}{} (EPSG:{code})",
            zone,
            if south { "S" } else { "N" }
        ),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, south)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

fn polar_stereographic_variant_b_scale(lat_ts_deg: f64, ellipsoid: &Ellipsoid) -> f64 {
    let phi = lat_ts_deg.abs().to_radians();
    let sin_phi = phi.sin();
    let e = ellipsoid.e;
    let m = phi.cos() / (1.0 - ellipsoid.e2 * sin_phi * sin_phi).sqrt();
    let t = (std::f64::consts::FRAC_PI_4 - 0.5 * phi).tan()
        / ((1.0 - e * sin_phi) / (1.0 + e * sin_phi)).powf(e / 2.0);
    let c = ((1.0 + e).powf(1.0 + e) * (1.0 - e).powf(1.0 - e)).sqrt();
    m * c / (2.0 * t)
}

fn mercator_variant_b_scale(lat_ts_deg: f64, ellipsoid: &Ellipsoid) -> f64 {
    let phi = lat_ts_deg.to_radians();
    let sin_phi = phi.sin();
    phi.cos() / (1.0 - ellipsoid.e2 * sin_phi * sin_phi).sqrt()
}

fn gauss_kruger(code: u32, lon0: f64, label: &str) -> Result<Crs> {
    let zone = ((lon0 / 3.0).round() as i32).unsigned_abs();
    Ok(Crs {
        name: format!("DHDN / 3-degree Gauss-Kruger {label} (EPSG:{code})"),
        datum: Datum::DHDN,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(zone as f64 * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::BESSEL),
        )?,
    })
}

fn ed50_gk_3deg_zone(code: u32, zone: u32) -> Result<Crs> {
    let lon0 = f64::from(zone) * 3.0;
    let false_easting = f64::from(zone) * 1_000_000.0 + 500_000.0;
    Ok(Crs {
        name: format!("ED50 / 3-degree GK zone {zone} (EPSG:{code})"),
        datum: Datum::ED50,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::INTERNATIONAL),
        )?,
    })
}

fn xian_1980_gk_crs(code: u32) -> Result<Crs> {
    let (name, lon0, false_easting) = match code {
        2334..=2337 => {
            let zone = code - 2314; // 20..23
            (
                format!("Xian 1980 / GK zone {zone} (EPSG:{code})"),
                6.0 * f64::from(zone) - 3.0,
                f64::from(zone) * 1_000_000.0 + 500_000.0,
            )
        }
        2338..=2348 => {
            let lon0 = 75.0 + 6.0 * f64::from(code - 2338);
            (
                format!("Xian 1980 / GK CM {lon0:.0}E (EPSG:{code})"),
                lon0,
                500_000.0,
            )
        }
        2349..=2369 => {
            let zone = code - 2324; // 25..45
            (
                format!("Xian 1980 / 3-degree GK zone {zone} (EPSG:{code})"),
                3.0 * f64::from(zone),
                f64::from(zone) * 1_000_000.0 + 500_000.0,
            )
        }
        2370..=2390 => {
            let lon0 = 75.0 + 3.0 * f64::from(code - 2370);
            (
                format!("Xian 1980 / 3-degree GK CM {lon0:.0}E (EPSG:{code})"),
                lon0,
                500_000.0,
            )
        }
        _ => {
            return Err(ProjectionError::UnsupportedProjection(format!(
                "EPSG:{code} is not in Xian 1980 GK family 2334-2390"
            )));
        }
    };

    let ellipsoid = Ellipsoid::from_a_inv_f("Xian 1980", 6_378_140.0, 298.257);
    Ok(Crs {
        name,
        datum: Datum {
            name: "Xian 1980",
            ellipsoid: ellipsoid.clone(),
            transform: DatumTransform::None,
        },
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(ellipsoid),
        )?,
    })
}

fn build_epsg_3580_3751(code: u32) -> Result<Crs> {
    match code {
        3580 => us_state_plane_lcc(3580, "NAD 1983 Northwest Territories Lambert", Datum::NAD83, -112.0, 62.0, 70.0, 0.0, 0.0, 0.0),
        3581 => us_state_plane_lcc(3581, "NAD 1983 CSRS Northwest Territories Lambert", Datum::NAD83_CSRS, -112.0, 62.0, 70.0, 0.0, 0.0, 0.0),
        3582 => us_state_plane_lcc_ftus(3582, "NAD 1983 NSRS2007 StatePlane Maryland FIPS 1900 Ft US", Datum::NAD83_NSRS2007, -77.0, 38.3, 39.45, 37.66666666666666, 1312333.333333333, 0.0),
        3583 => us_state_plane_lcc(3583, "NAD 1983 NSRS2007 StatePlane Massachusetts Island FIPS 2002", Datum::NAD83_NSRS2007, -70.5, 41.28333333333333, 41.48333333333333, 41.0, 500000.0, 0.0),
        3584 => us_state_plane_lcc_ftus(3584, "NAD 1983 NSRS2007 StatePlane Massachusetts Isl FIPS 2002 FtUS", Datum::NAD83_NSRS2007, -70.5, 41.28333333333333, 41.48333333333333, 41.0, 1640416.666666667, 0.0),
        3585 => us_state_plane_lcc(3585, "NAD 1983 NSRS2007 StatePlane Massachusetts Mainland FIPS 2001", Datum::NAD83_NSRS2007, -71.5, 41.71666666666667, 42.68333333333333, 41.0, 200000.0, 750000.0),
        3586 => us_state_plane_lcc_ftus(3586, "NAD 1983 NSRS2007 StatePlane Massachusetts Mnld FIPS 2001 FtUS", Datum::NAD83_NSRS2007, -71.5, 41.71666666666667, 42.68333333333333, 41.0, 656166.6666666665, 2460625.0),
        3587 => us_state_plane_lcc(3587, "NAD 1983 NSRS2007 StatePlane Michigan Central FIPS 2112", Datum::NAD83_NSRS2007, -84.36666666666666, 44.18333333333333, 45.7, 43.31666666666667, 6000000.0, 0.0),
        3588 => us_state_plane_lcc_ft(3588, "NAD 1983 NSRS2007 StatePlane Michigan Central FIPS 2112 Ft Intl", Datum::NAD83_NSRS2007, -84.36666666666666, 44.18333333333333, 45.7, 43.31666666666667, 19685039.37007874, 0.0),
        3589 => us_state_plane_lcc(3589, "NAD 1983 NSRS2007 StatePlane Michigan North FIPS 2111", Datum::NAD83_NSRS2007, -87.0, 45.48333333333333, 47.08333333333334, 44.78333333333333, 8000000.0, 0.0),
        3590 => us_state_plane_lcc_ft(3590, "NAD 1983 NSRS2007 StatePlane Michigan North FIPS 2111 Ft Intl", Datum::NAD83_NSRS2007, -87.0, 45.48333333333333, 47.08333333333334, 44.78333333333333, 26246719.16010498, 0.0),
        3591 => us_state_plane_omerc(3591, "NAD 1983 NSRS2007 Michigan GeoRef Meters", Datum::NAD83_NSRS2007, -86.0, 45.30916666666666, 337.25556, 0.9996, 2546731.496, -4354009.816),
        3592 => us_state_plane_lcc(3592, "NAD 1983 NSRS2007 StatePlane Michigan South FIPS 2113", Datum::NAD83_NSRS2007, -84.36666666666666, 42.1, 43.66666666666666, 41.5, 4000000.0, 0.0),
        3593 => us_state_plane_lcc_ft(3593, "NAD 1983 NSRS2007 StatePlane Michigan South FIPS 2113 Ft Intl", Datum::NAD83_NSRS2007, -84.36666666666666, 42.1, 43.66666666666666, 41.5, 13123359.58005249, 0.0),
        3594 => us_state_plane_lcc(3594, "NAD 1983 NSRS2007 StatePlane Minnesota Central FIPS 2202", Datum::NAD83_NSRS2007, -94.25, 45.61666666666667, 47.05, 45.0, 800000.0, 100000.0),
        3595 => us_state_plane_lcc(3595, "NAD 1983 NSRS2007 StatePlane Minnesota North FIPS 2201", Datum::NAD83_NSRS2007, -93.1, 47.03333333333333, 48.63333333333333, 46.5, 800000.0, 100000.0),
        3596 => us_state_plane_lcc(3596, "NAD 1983 NSRS2007 StatePlane Minnesota South FIPS 2203", Datum::NAD83_NSRS2007, -94.0, 43.78333333333333, 45.21666666666667, 43.0, 800000.0, 100000.0),
        3597 => us_state_plane_tm(3597, "NAD 1983 NSRS2007 StatePlane Mississippi East FIPS 2301", Datum::NAD83_NSRS2007, -88.83333333333333, 29.5, 0.99995, 300000.0, 0.0),
        3598 => us_state_plane_tm_ftus(3598, "NAD 1983 NSRS2007 StatePlane Mississippi East FIPS 2301 Ft US", Datum::NAD83_NSRS2007, -88.83333333333333, 29.5, 0.99995, 984250.0, 0.0),
        3599 => us_state_plane_tm(3599, "NAD 1983 NSRS2007 StatePlane Mississippi West FIPS 2302", Datum::NAD83_NSRS2007, -90.33333333333333, 29.5, 0.99995, 700000.0, 0.0),
        3600 => us_state_plane_tm_ftus(3600, "NAD 1983 NSRS2007 StatePlane Mississippi West FIPS 2302 Ft US", Datum::NAD83_NSRS2007, -90.33333333333333, 29.5, 0.99995, 2296583.333333333, 0.0),
        3601 => us_state_plane_tm(3601, "NAD 1983 NSRS2007 StatePlane Missouri Central FIPS 2402", Datum::NAD83_NSRS2007, -92.5, 35.83333333333334, 0.9999333333333333, 500000.0, 0.0),
        3602 => us_state_plane_tm(3602, "NAD 1983 NSRS2007 StatePlane Missouri East FIPS 2401", Datum::NAD83_NSRS2007, -90.5, 35.83333333333334, 0.9999333333333333, 250000.0, 0.0),
        3603 => us_state_plane_tm(3603, "NAD 1983 NSRS2007 StatePlane Missouri West FIPS 2403", Datum::NAD83_NSRS2007, -94.5, 36.16666666666666, 0.9999411764705882, 850000.0, 0.0),
        3604 => us_state_plane_lcc(3604, "NAD 1983 NSRS2007 StatePlane Montana FIPS 2500", Datum::NAD83_NSRS2007, -109.5, 45.0, 49.0, 44.25, 600000.0, 0.0),
        3605 => us_state_plane_lcc_ft(3605, "NAD 1983 NSRS2007 StatePlane Montana FIPS 2500 Ft Intl", Datum::NAD83_NSRS2007, -109.5, 45.0, 49.0, 44.25, 1968503.937007874, 0.0),
        3606 => us_state_plane_lcc(3606, "NAD 1983 NSRS2007 StatePlane Nebraska FIPS 2600", Datum::NAD83_NSRS2007, -100.0, 40.0, 43.0, 39.83333333333334, 500000.0, 0.0),
        3607 => us_state_plane_tm(3607, "NAD 1983 NSRS2007 StatePlane Nevada Central FIPS 2702", Datum::NAD83_NSRS2007, -116.6666666666667, 34.75, 0.9999, 500000.0, 6000000.0),
        3608 => us_state_plane_tm_ftus(3608, "NAD 1983 NSRS2007 StatePlane Nevada Central FIPS 2702 Ft US", Datum::NAD83_NSRS2007, -116.6666666666667, 34.75, 0.9999, 1640416.666666667, 19685000.0),
        3609 => us_state_plane_tm(3609, "NAD 1983 NSRS2007 StatePlane Nevada East FIPS 2701", Datum::NAD83_NSRS2007, -115.5833333333333, 34.75, 0.9999, 200000.0, 8000000.0),
        3610 => us_state_plane_tm_ftus(3610, "NAD 1983 NSRS2007 StatePlane Nevada East FIPS 2701 Ft US", Datum::NAD83_NSRS2007, -115.5833333333333, 34.75, 0.9999, 656166.6666666665, 26246666.66666666),
        3611 => us_state_plane_tm(3611, "NAD 1983 NSRS2007 StatePlane Nevada West FIPS 2703", Datum::NAD83_NSRS2007, -118.5833333333333, 34.75, 0.9999, 800000.0, 4000000.0),
        3612 => us_state_plane_tm_ftus(3612, "NAD 1983 NSRS2007 StatePlane Nevada West FIPS 2703 Ft US", Datum::NAD83_NSRS2007, -118.5833333333333, 34.75, 0.9999, 2624666.666666666, 13123333.33333333),
        3613 => us_state_plane_tm(3613, "NAD 1983 NSRS2007 StatePlane New Hampshire FIPS 2800", Datum::NAD83_NSRS2007, -71.66666666666667, 42.5, 0.9999666666666667, 300000.0, 0.0),
        3614 => us_state_plane_tm_ftus(3614, "NAD 1983 NSRS2007 StatePlane New Hampshire FIPS 2800 Ft US", Datum::NAD83_NSRS2007, -71.66666666666667, 42.5, 0.9999666666666667, 984250.0, 0.0),
        3615 => us_state_plane_tm(3615, "NAD 1983 NSRS2007 StatePlane New Jersey FIPS 2900", Datum::NAD83_NSRS2007, -74.5, 38.83333333333334, 0.9999, 150000.0, 0.0),
        3616 => us_state_plane_tm_ftus(3616, "NAD 1983 NSRS2007 StatePlane New Jersey FIPS 2900 Ft US", Datum::NAD83_NSRS2007, -74.5, 38.83333333333334, 0.9999, 492125.0, 0.0),
        3617 => us_state_plane_tm(3617, "NAD 1983 NSRS2007 StatePlane New Mexico Central FIPS 3002", Datum::NAD83_NSRS2007, -106.25, 31.0, 0.9999, 500000.0, 0.0),
        3618 => us_state_plane_tm_ftus(3618, "NAD 1983 NSRS2007 StatePlane New Mexico Central FIPS 3002 Ft US", Datum::NAD83_NSRS2007, -106.25, 31.0, 0.9999, 1640416.666666667, 0.0),
        3619 => us_state_plane_tm(3619, "NAD 1983 NSRS2007 StatePlane New Mexico East FIPS 3001", Datum::NAD83_NSRS2007, -104.3333333333333, 31.0, 0.9999090909090909, 165000.0, 0.0),
        3620 => us_state_plane_tm_ftus(3620, "NAD 1983 NSRS2007 StatePlane New Mexico East FIPS 3001 Ft US", Datum::NAD83_NSRS2007, -104.3333333333333, 31.0, 0.9999090909090909, 541337.5, 0.0),
        3621 => us_state_plane_tm(3621, "NAD 1983 NSRS2007 StatePlane New Mexico West FIPS 3003", Datum::NAD83_NSRS2007, -107.8333333333333, 31.0, 0.9999166666666667, 830000.0, 0.0),
        3622 => us_state_plane_tm_ftus(3622, "NAD 1983 NSRS2007 StatePlane New Mexico West FIPS 3003 Ft US", Datum::NAD83_NSRS2007, -107.8333333333333, 31.0, 0.9999166666666667, 2723091.666666666, 0.0),
        3623 => us_state_plane_tm(3623, "NAD 1983 NSRS2007 StatePlane New York Central FIPS 3102", Datum::NAD83_NSRS2007, -76.58333333333333, 40.0, 0.9999375, 250000.0, 0.0),
        3624 => us_state_plane_tm_ftus(3624, "NAD 1983 NSRS2007 StatePlane New York Central FIPS 3102 Ft US", Datum::NAD83_NSRS2007, -76.58333333333333, 40.0, 0.9999375, 820208.3333333333, 0.0),
        3625 => us_state_plane_tm(3625, "NAD 1983 NSRS2007 StatePlane New York East FIPS 3101", Datum::NAD83_NSRS2007, -74.5, 38.83333333333334, 0.9999, 150000.0, 0.0),
        3626 => us_state_plane_tm_ftus(3626, "NAD 1983 NSRS2007 StatePlane New York East FIPS 3101 Ft US", Datum::NAD83_NSRS2007, -74.5, 38.83333333333334, 0.9999, 492125.0, 0.0),
        3627 => us_state_plane_lcc(3627, "NAD 1983 NSRS2007 StatePlane New York Long Island FIPS 3104", Datum::NAD83_NSRS2007, -74.0, 40.66666666666666, 41.03333333333333, 40.16666666666666, 300000.0, 0.0),
        3628 => us_state_plane_lcc_ftus(3628, "NAD 1983 NSRS2007 StatePlane New York Long Isl FIPS 3104 Ft US", Datum::NAD83_NSRS2007, -74.0, 40.66666666666666, 41.03333333333333, 40.16666666666666, 984250.0, 0.0),
        3629 => us_state_plane_tm(3629, "NAD 1983 NSRS2007 StatePlane New York West FIPS 3103", Datum::NAD83_NSRS2007, -78.58333333333333, 40.0, 0.9999375, 350000.0, 0.0),
        3630 => us_state_plane_tm_ftus(3630, "NAD 1983 NSRS2007 StatePlane New York West FIPS 3103 Ft US", Datum::NAD83_NSRS2007, -78.58333333333333, 40.0, 0.9999375, 1148291.666666667, 0.0),
        3631 => us_state_plane_lcc(3631, "NAD 1983 NSRS2007 StatePlane North Carolina FIPS 3200", Datum::NAD83_NSRS2007, -79.0, 34.33333333333334, 36.16666666666666, 33.75, 609601.2192024384, 0.0),
        3632 => us_state_plane_lcc_ftus(3632, "NAD 1983 NSRS2007 StatePlane North Carolina FIPS 3200 Ft US", Datum::NAD83_NSRS2007, -79.0, 34.33333333333334, 36.16666666666666, 33.75, 2000000.0, 0.0),
        3633 => us_state_plane_lcc(3633, "NAD 1983 NSRS2007 StatePlane North Dakota North FIPS 3301", Datum::NAD83_NSRS2007, -100.5, 47.43333333333333, 48.73333333333333, 47.0, 600000.0, 0.0),
        3634 => us_state_plane_lcc_ft(3634, "NAD 1983 NSRS2007 StatePlane North Dakota North FIPS 3301 FtI", Datum::NAD83_NSRS2007, -100.5, 47.43333333333333, 48.73333333333333, 47.0, 1968503.937007874, 0.0),
        3635 => us_state_plane_lcc(3635, "NAD 1983 NSRS2007 StatePlane North Dakota South FIPS 3302", Datum::NAD83_NSRS2007, -100.5, 46.18333333333333, 47.48333333333333, 45.66666666666666, 600000.0, 0.0),
        3636 => us_state_plane_lcc_ft(3636, "NAD 1983 NSRS2007 StatePlane North Dakota South FIPS 3302 FtI", Datum::NAD83_NSRS2007, -100.5, 46.18333333333333, 47.48333333333333, 45.66666666666666, 1968503.937007874, 0.0),
        3637 => us_state_plane_lcc(3637, "NAD 1983 NSRS2007 StatePlane Ohio North FIPS 3401", Datum::NAD83_NSRS2007, -82.5, 40.43333333333333, 41.7, 39.66666666666666, 600000.0, 0.0),
        3638 => us_state_plane_lcc(3638, "NAD 1983 NSRS2007 StatePlane Ohio South FIPS 3402", Datum::NAD83_NSRS2007, -82.5, 38.73333333333333, 40.03333333333333, 38.0, 600000.0, 0.0),
        3639 => us_state_plane_lcc(3639, "NAD 1983 NSRS2007 StatePlane Oklahoma North FIPS 3501", Datum::NAD83_NSRS2007, -98.0, 35.56666666666667, 36.76666666666667, 35.0, 600000.0, 0.0),
        3640 => us_state_plane_lcc_ftus(3640, "NAD 1983 NSRS2007 StatePlane Oklahoma North FIPS 3501 Ft US", Datum::NAD83_NSRS2007, -98.0, 35.56666666666667, 36.76666666666667, 35.0, 1968500.0, 0.0),
        3641 => us_state_plane_lcc(3641, "NAD 1983 NSRS2007 StatePlane Oklahoma South FIPS 3502", Datum::NAD83_NSRS2007, -98.0, 33.93333333333333, 35.23333333333333, 33.33333333333334, 600000.0, 0.0),
        3642 => us_state_plane_lcc_ftus(3642, "NAD 1983 NSRS2007 StatePlane Oklahoma South FIPS 3502 Ft US", Datum::NAD83_NSRS2007, -98.0, 33.93333333333333, 35.23333333333333, 33.33333333333334, 1968500.0, 0.0),
        3643 => us_state_plane_lcc(3643, "NAD 1983 NSRS2007 Oregon Statewide Lambert", Datum::NAD83_NSRS2007, -120.5, 43.0, 45.5, 41.75, 400000.0, 0.0),
        3644 => us_state_plane_lcc_ft(3644, "NAD 1983 NSRS2007 Oregon Statewide Lambert Ft Intl", Datum::NAD83_NSRS2007, -120.5, 43.0, 45.5, 41.75, 1312335.958005249, 0.0),
        3645 => us_state_plane_lcc(3645, "NAD 1983 NSRS2007 StatePlane Oregon North FIPS 3601", Datum::NAD83_NSRS2007, -120.5, 44.33333333333334, 46.0, 43.66666666666666, 2500000.0, 0.0),
        3646 => us_state_plane_lcc_ft(3646, "NAD 1983 NSRS2007 StatePlane Oregon North FIPS 3601 Ft Intl", Datum::NAD83_NSRS2007, -120.5, 44.33333333333334, 46.0, 43.66666666666666, 8202099.737532808, 0.0),
        3647 => us_state_plane_lcc(3647, "NAD 1983 NSRS2007 StatePlane Oregon South FIPS 3602", Datum::NAD83_NSRS2007, -120.5, 42.33333333333334, 44.0, 41.66666666666666, 1500000.0, 0.0),
        3648 => us_state_plane_lcc_ft(3648, "NAD 1983 NSRS2007 StatePlane Oregon South FIPS 3602 Ft Intl", Datum::NAD83_NSRS2007, -120.5, 42.33333333333334, 44.0, 41.66666666666666, 4921259.842519685, 0.0),
        3649 => us_state_plane_lcc(3649, "NAD 1983 NSRS2007 StatePlane Pennsylvania North FIPS 3701", Datum::NAD83_NSRS2007, -77.75, 40.88333333333333, 41.95, 40.16666666666666, 600000.0, 0.0),
        3650 => us_state_plane_lcc_ftus(3650, "NAD 1983 NSRS2007 StatePlane Pennsylvania North FIPS 3701 Ft US", Datum::NAD83_NSRS2007, -77.75, 40.88333333333333, 41.95, 40.16666666666666, 1968500.0, 0.0),
        3651 => us_state_plane_lcc(3651, "NAD 1983 NSRS2007 StatePlane Pennsylvania South FIPS 3702", Datum::NAD83_NSRS2007, -77.75, 39.93333333333333, 40.96666666666667, 39.33333333333334, 600000.0, 0.0),
        3652 => us_state_plane_lcc_ftus(3652, "NAD 1983 NSRS2007 StatePlane Pennsylvania South FIPS 3702 Ft US", Datum::NAD83_NSRS2007, -77.75, 39.93333333333333, 40.96666666666667, 39.33333333333334, 1968500.0, 0.0),
        3653 => us_state_plane_tm(3653, "NAD 1983 NSRS2007 StatePlane Rhode Island FIPS 3800", Datum::NAD83_NSRS2007, -71.5, 41.08333333333334, 0.99999375, 100000.0, 0.0),
        3654 => us_state_plane_tm_ftus(3654, "NAD 1983 NSRS2007 StatePlane Rhode Island FIPS 3800 Ft US", Datum::NAD83_NSRS2007, -71.5, 41.08333333333334, 0.99999375, 328083.3333333333, 0.0),
        3655 => us_state_plane_lcc(3655, "NAD 1983 NSRS2007 StatePlane South Carolina FIPS 3900", Datum::NAD83_NSRS2007, -81.0, 32.5, 34.83333333333334, 31.83333333333333, 609600.0, 0.0),
        3656 => us_state_plane_lcc_ft(3656, "NAD 1983 NSRS2007 StatePlane South Carolina FIPS 3900 Ft Intl", Datum::NAD83_NSRS2007, -81.0, 32.5, 34.83333333333334, 31.83333333333333, 2000000.0, 0.0),
        3657 => us_state_plane_lcc(3657, "NAD 1983 NSRS2007 StatePlane South Dakota North FIPS 4001", Datum::NAD83_NSRS2007, -100.0, 44.41666666666666, 45.68333333333333, 43.83333333333334, 600000.0, 0.0),
        3658 => us_state_plane_lcc_ftus(3658, "NAD 1983 NSRS2007 StatePlane South Dakota North FIPS 4001 Ft US", Datum::NAD83_NSRS2007, -100.0, 44.41666666666666, 45.68333333333333, 43.83333333333334, 1968500.0, 0.0),
        3659 => us_state_plane_lcc(3659, "NAD 1983 NSRS2007 StatePlane South Dakota South FIPS 4002", Datum::NAD83_NSRS2007, -100.3333333333333, 42.83333333333334, 44.4, 42.33333333333334, 600000.0, 0.0),
        3660 => us_state_plane_lcc_ftus(3660, "NAD 1983 NSRS2007 StatePlane South Dakota South FIPS 4002 Ft US", Datum::NAD83_NSRS2007, -100.3333333333333, 42.83333333333334, 44.4, 42.33333333333334, 1968500.0, 0.0),
        3661 => us_state_plane_lcc(3661, "NAD 1983 NSRS2007 StatePlane Tennessee FIPS 4100", Datum::NAD83_NSRS2007, -86.0, 35.25, 36.41666666666666, 34.33333333333334, 600000.0, 0.0),
        3662 => us_state_plane_lcc_ftus(3662, "NAD 1983 NSRS2007 StatePlane Tennessee FIPS 4100 Ft US", Datum::NAD83_NSRS2007, -86.0, 35.25, 36.41666666666666, 34.33333333333334, 1968500.0, 0.0),
        3663 => us_state_plane_lcc(3663, "NAD 1983 NSRS2007 StatePlane Texas Central FIPS 4203", Datum::NAD83_NSRS2007, -100.3333333333333, 30.11666666666667, 31.88333333333333, 29.66666666666667, 700000.0, 3000000.0),
        3664 => us_state_plane_lcc_ftus(3664, "NAD 1983 NSRS2007 StatePlane Texas Central FIPS 4203 Ft US", Datum::NAD83_NSRS2007, -100.3333333333333, 30.11666666666667, 31.88333333333333, 29.66666666666667, 2296583.333333333, 9842500.0),
        3665 => Ok(Crs { name: "NAD 1983 NSRS2007 Texas Centric Mapping System Albers (EPSG:3665)".into(), datum: Datum::NAD83_NSRS2007.clone(), projection: crate::projections::Projection::new(ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic { lat1: 27.5, lat2: 35.0 }).with_lon0(-100.0).with_lat0(18.0).with_false_easting(1500000.0).with_false_northing(6000000.0).with_ellipsoid(Datum::NAD83_NSRS2007.ellipsoid.clone()))?, }),
        3666 => us_state_plane_lcc(3666, "NAD 1983 NSRS2007 Texas Centric Mapping System Lambert", Datum::NAD83_NSRS2007, -100.0, 27.5, 35.0, 18.0, 1500000.0, 5000000.0),
        3667 => us_state_plane_lcc(3667, "NAD 1983 NSRS2007 StatePlane Texas North FIPS 4201", Datum::NAD83_NSRS2007, -101.5, 34.65, 36.18333333333333, 34.0, 200000.0, 1000000.0),
        3668 => us_state_plane_lcc_ftus(3668, "NAD 1983 NSRS2007 StatePlane Texas North FIPS 4201 Ft US", Datum::NAD83_NSRS2007, -101.5, 34.65, 36.18333333333333, 34.0, 656166.6666666665, 3280833.333333333),
        3669 => us_state_plane_lcc(3669, "NAD 1983 NSRS2007 StatePlane Texas North Central FIPS 4202", Datum::NAD83_NSRS2007, -98.5, 32.13333333333333, 33.96666666666667, 31.66666666666667, 600000.0, 2000000.0),
        3670 => us_state_plane_lcc_ftus(3670, "NAD 1983 NSRS2007 StatePlane Texas North Central FIPS 4202 FtUS", Datum::NAD83_NSRS2007, -98.5, 32.13333333333333, 33.96666666666667, 31.66666666666667, 1968500.0, 6561666.666666666),
        3671 => us_state_plane_lcc(3671, "NAD 1983 NSRS2007 StatePlane Texas South FIPS 4205", Datum::NAD83_NSRS2007, -98.5, 26.16666666666667, 27.83333333333333, 25.66666666666667, 300000.0, 5000000.0),
        3672 => us_state_plane_lcc_ftus(3672, "NAD 1983 NSRS2007 StatePlane Texas South FIPS 4205 Ft US", Datum::NAD83_NSRS2007, -98.5, 26.16666666666667, 27.83333333333333, 25.66666666666667, 984250.0, 16404166.66666666),
        3673 => us_state_plane_lcc(3673, "NAD 1983 NSRS2007 StatePlane Texas South Central FIPS 4204", Datum::NAD83_NSRS2007, -99.0, 28.38333333333333, 30.28333333333333, 27.83333333333333, 600000.0, 4000000.0),
        3674 => us_state_plane_lcc_ftus(3674, "NAD 1983 NSRS2007 StatePlane Texas South Central FIPS 4204 FtUS", Datum::NAD83_NSRS2007, -99.0, 28.38333333333333, 30.28333333333333, 27.83333333333333, 1968500.0, 13123333.33333333),
        3675 => us_state_plane_lcc(3675, "NAD 1983 NSRS2007 StatePlane Utah Central FIPS 4302", Datum::NAD83_NSRS2007, -111.5, 39.01666666666667, 40.65, 38.33333333333334, 500000.0, 2000000.0),
        3676 => us_state_plane_lcc_ft(3676, "NAD 1983 NSRS2007 StatePlane Utah Central FIPS 4302 Ft Intl", Datum::NAD83_NSRS2007, -111.5, 39.01666666666667, 40.65, 38.33333333333334, 1640419.947506561, 6561679.790026246),
        3677 => us_state_plane_lcc_ftus(3677, "NAD 1983 NSRS2007 StatePlane Utah Central FIPS 4302 Ft US", Datum::NAD83_NSRS2007, -111.5, 39.01666666666667, 40.65, 38.33333333333334, 1640416.666666667, 6561666.666666666),
        3678 => us_state_plane_lcc(3678, "NAD 1983 NSRS2007 StatePlane Utah North FIPS 4301", Datum::NAD83_NSRS2007, -111.5, 40.71666666666667, 41.78333333333333, 40.33333333333334, 500000.0, 1000000.0),
        3679 => us_state_plane_lcc_ft(3679, "NAD 1983 NSRS2007 StatePlane Utah North FIPS 4301 Ft Intl", Datum::NAD83_NSRS2007, -111.5, 40.71666666666667, 41.78333333333333, 40.33333333333334, 1640419.947506561, 3280839.895013123),
        3680 => us_state_plane_lcc_ftus(3680, "NAD 1983 NSRS2007 StatePlane Utah North FIPS 4301 Ft US", Datum::NAD83_NSRS2007, -111.5, 40.71666666666667, 41.78333333333333, 40.33333333333334, 1640416.666666667, 3280833.333333333),
        3681 => us_state_plane_lcc(3681, "NAD 1983 NSRS2007 StatePlane Utah South FIPS 4303", Datum::NAD83_NSRS2007, -111.5, 37.21666666666667, 38.35, 36.66666666666666, 500000.0, 3000000.0),
        3682 => us_state_plane_lcc_ft(3682, "NAD 1983 NSRS2007 StatePlane Utah South FIPS 4303 Ft Intl", Datum::NAD83_NSRS2007, -111.5, 37.21666666666667, 38.35, 36.66666666666666, 1640419.947506561, 9842519.685039369),
        3683 => us_state_plane_lcc_ftus(3683, "NAD 1983 NSRS2007 StatePlane Utah South FIPS 4303 Ft US", Datum::NAD83_NSRS2007, -111.5, 37.21666666666667, 38.35, 36.66666666666666, 1640416.666666667, 9842500.0),
        3684 => us_state_plane_tm(3684, "NAD 1983 NSRS2007 StatePlane Vermont FIPS 4400", Datum::NAD83_NSRS2007, -72.5, 42.5, 0.9999642857142857, 500000.0, 0.0),
        3685 => us_state_plane_lcc(3685, "NAD 1983 NSRS2007 StatePlane Virginia North FIPS 4501", Datum::NAD83_NSRS2007, -78.5, 38.03333333333333, 39.2, 37.66666666666666, 3500000.0, 2000000.0),
        3686 => us_state_plane_lcc_ftus(3686, "NAD 1983 NSRS2007 StatePlane Virginia North FIPS 4501 Ft US", Datum::NAD83_NSRS2007, -78.5, 38.03333333333333, 39.2, 37.66666666666666, 11482916.66666666, 6561666.666666666),
        3687 => us_state_plane_lcc(3687, "NAD 1983 NSRS2007 StatePlane Virginia South FIPS 4502", Datum::NAD83_NSRS2007, -78.5, 36.76666666666667, 37.96666666666667, 36.33333333333334, 3500000.0, 1000000.0),
        3688 => us_state_plane_lcc_ftus(3688, "NAD 1983 NSRS2007 StatePlane Virginia South FIPS 4502 Ft US", Datum::NAD83_NSRS2007, -78.5, 36.76666666666667, 37.96666666666667, 36.33333333333334, 11482916.66666666, 3280833.333333333),
        3689 => us_state_plane_lcc(3689, "NAD 1983 NSRS2007 StatePlane Washington North FIPS 4601", Datum::NAD83_NSRS2007, -120.8333333333333, 47.5, 48.73333333333333, 47.0, 500000.0, 0.0),
        3690 => us_state_plane_lcc_ftus(3690, "NAD 1983 NSRS2007 StatePlane Washington North FIPS 4601 Ft US", Datum::NAD83_NSRS2007, -120.8333333333333, 47.5, 48.73333333333333, 47.0, 1640416.666666667, 0.0),
        3691 => us_state_plane_lcc(3691, "NAD 1983 NSRS2007 StatePlane Washington South FIPS 4602", Datum::NAD83_NSRS2007, -120.5, 45.83333333333334, 47.33333333333334, 45.33333333333334, 500000.0, 0.0),
        3692 => us_state_plane_lcc_ftus(3692, "NAD 1983 NSRS2007 StatePlane Washington South FIPS 4602 Ft US", Datum::NAD83_NSRS2007, -120.5, 45.83333333333334, 47.33333333333334, 45.33333333333334, 1640416.666666667, 0.0),
        3693 => us_state_plane_lcc(3693, "NAD 1983 NSRS2007 StatePlane West Virginia North FIPS 4701", Datum::NAD83_NSRS2007, -79.5, 39.0, 40.25, 38.5, 600000.0, 0.0),
        3694 => us_state_plane_lcc(3694, "NAD 1983 NSRS2007 StatePlane West Virginia South FIPS 4702", Datum::NAD83_NSRS2007, -81.0, 37.48333333333333, 38.88333333333333, 37.0, 600000.0, 0.0),
        3695 => us_state_plane_lcc(3695, "NAD 1983 NSRS2007 StatePlane Wisconsin Central FIPS 4802", Datum::NAD83_NSRS2007, -90.0, 44.25, 45.5, 43.83333333333334, 600000.0, 0.0),
        3696 => us_state_plane_lcc_ftus(3696, "NAD 1983 NSRS2007 StatePlane Wisconsin Central FIPS 4802 Ft US", Datum::NAD83_NSRS2007, -90.0, 44.25, 45.5, 43.83333333333334, 1968500.0, 0.0),
        3697 => us_state_plane_lcc(3697, "NAD 1983 NSRS2007 StatePlane Wisconsin North FIPS 4801", Datum::NAD83_NSRS2007, -90.0, 45.56666666666667, 46.76666666666667, 45.16666666666666, 600000.0, 0.0),
        3698 => us_state_plane_lcc_ftus(3698, "NAD 1983 NSRS2007 StatePlane Wisconsin North FIPS 4801 Ft US", Datum::NAD83_NSRS2007, -90.0, 45.56666666666667, 46.76666666666667, 45.16666666666666, 1968500.0, 0.0),
        3699 => us_state_plane_lcc(3699, "NAD 1983 NSRS2007 StatePlane Wisconsin South FIPS 4803", Datum::NAD83_NSRS2007, -90.0, 42.73333333333333, 44.06666666666667, 42.0, 600000.0, 0.0),
        3700 => us_state_plane_lcc_ftus(3700, "NAD 1983 NSRS2007 StatePlane Wisconsin South FIPS 4803 Ft US", Datum::NAD83_NSRS2007, -90.0, 42.73333333333333, 44.06666666666667, 42.0, 1968500.0, 0.0),
        3701 => us_state_plane_tm(3701, "NAD 1983 NSRS2007 Wisconsin TM", Datum::NAD83_NSRS2007, -90.0, 0.0, 0.9996, 520000.0, -4480000.0),
        3702 => us_state_plane_tm(3702, "NAD 1983 NSRS2007 StatePlane Wyoming East FIPS 4901", Datum::NAD83_NSRS2007, -105.1666666666667, 40.5, 0.9999375, 200000.0, 0.0),
        3703 => us_state_plane_tm(3703, "NAD 1983 NSRS2007 StatePlane Wyoming East Central FIPS 4902", Datum::NAD83_NSRS2007, -107.3333333333333, 40.5, 0.9999375, 400000.0, 100000.0),
        3704 => us_state_plane_tm(3704, "NAD 1983 NSRS2007 StatePlane Wyoming West Central FIPS 4903", Datum::NAD83_NSRS2007, -108.75, 40.5, 0.9999375, 600000.0, 0.0),
        3705 => us_state_plane_tm(3705, "NAD 1983 NSRS2007 StatePlane Wyoming West FIPS 4904", Datum::NAD83_NSRS2007, -110.0833333333333, 40.5, 0.9999375, 800000.0, 100000.0),
        3706 => us_state_plane_tm(3706, "NAD 1983 NSRS2007 UTM Zone 59N", Datum::NAD83_NSRS2007, 171.0, 0.0, 0.9996, 500000.0, 0.0),
        3707 => us_state_plane_tm(3707, "NAD 1983 NSRS2007 UTM Zone 60N", Datum::NAD83_NSRS2007, 177.0, 0.0, 0.9996, 500000.0, 0.0),
        3708 => us_state_plane_tm(3708, "NAD 1983 NSRS2007 UTM Zone 1N", Datum::NAD83_NSRS2007, -177.0, 0.0, 0.9996, 500000.0, 0.0),
        3709 => us_state_plane_tm(3709, "NAD 1983 NSRS2007 UTM Zone 2N", Datum::NAD83_NSRS2007, -171.0, 0.0, 0.9996, 500000.0, 0.0),
        3710 => us_state_plane_tm(3710, "NAD 1983 NSRS2007 UTM Zone 3N", Datum::NAD83_NSRS2007, -165.0, 0.0, 0.9996, 500000.0, 0.0),
        3711 => us_state_plane_tm(3711, "NAD 1983 NSRS2007 UTM Zone 4N", Datum::NAD83_NSRS2007, -159.0, 0.0, 0.9996, 500000.0, 0.0),
        3712 => us_state_plane_tm(3712, "NAD 1983 NSRS2007 UTM Zone 5N", Datum::NAD83_NSRS2007, -153.0, 0.0, 0.9996, 500000.0, 0.0),
        3713 => us_state_plane_tm(3713, "NAD 1983 NSRS2007 UTM Zone 6N", Datum::NAD83_NSRS2007, -147.0, 0.0, 0.9996, 500000.0, 0.0),
        3714 => us_state_plane_tm(3714, "NAD 1983 NSRS2007 UTM Zone 7N", Datum::NAD83_NSRS2007, -141.0, 0.0, 0.9996, 500000.0, 0.0),
        3715 => us_state_plane_tm(3715, "NAD 1983 NSRS2007 UTM Zone 8N", Datum::NAD83_NSRS2007, -135.0, 0.0, 0.9996, 500000.0, 0.0),
        3716 => us_state_plane_tm(3716, "NAD 1983 NSRS2007 UTM Zone 9N", Datum::NAD83_NSRS2007, -129.0, 0.0, 0.9996, 500000.0, 0.0),
        3717 => us_state_plane_tm(3717, "NAD 1983 NSRS2007 UTM Zone 10N", Datum::NAD83_NSRS2007, -123.0, 0.0, 0.9996, 500000.0, 0.0),
        3718 => us_state_plane_tm(3718, "NAD 1983 NSRS2007 UTM Zone 11N", Datum::NAD83_NSRS2007, -117.0, 0.0, 0.9996, 500000.0, 0.0),
        3719 => us_state_plane_tm(3719, "NAD 1983 NSRS2007 UTM Zone 12N", Datum::NAD83_NSRS2007, -111.0, 0.0, 0.9996, 500000.0, 0.0),
        3720 => us_state_plane_tm(3720, "NAD 1983 NSRS2007 UTM Zone 13N", Datum::NAD83_NSRS2007, -105.0, 0.0, 0.9996, 500000.0, 0.0),
        3721 => us_state_plane_tm(3721, "NAD 1983 NSRS2007 UTM Zone 14N", Datum::NAD83_NSRS2007, -99.0, 0.0, 0.9996, 500000.0, 0.0),
        3722 => us_state_plane_tm(3722, "NAD 1983 NSRS2007 UTM Zone 15N", Datum::NAD83_NSRS2007, -93.0, 0.0, 0.9996, 500000.0, 0.0),
        3723 => us_state_plane_tm(3723, "NAD 1983 NSRS2007 UTM Zone 16N", Datum::NAD83_NSRS2007, -87.0, 0.0, 0.9996, 500000.0, 0.0),
        3724 => us_state_plane_tm(3724, "NAD 1983 NSRS2007 UTM Zone 17N", Datum::NAD83_NSRS2007, -81.0, 0.0, 0.9996, 500000.0, 0.0),
        3725 => us_state_plane_tm(3725, "NAD 1983 NSRS2007 UTM Zone 18N", Datum::NAD83_NSRS2007, -75.0, 0.0, 0.9996, 500000.0, 0.0),
        3726 => us_state_plane_tm(3726, "NAD 1983 NSRS2007 UTM Zone 19N", Datum::NAD83_NSRS2007, -69.0, 0.0, 0.9996, 500000.0, 0.0),
        3727 => us_state_plane_tm(3727, "Reunion 1947 TM Reunion", Datum { name: "Reunion 1947", ellipsoid: Ellipsoid::INTERNATIONAL, transform: DatumTransform::None }, 55.53333333333333, -21.11666666666667, 1.0, 160000.0, 50000.0),
        3728 => us_state_plane_lcc_ftus(3728, "NAD 1983 NSRS2007 StatePlane Ohio North FIPS 3401 Ft US", Datum::NAD83_NSRS2007, -82.5, 40.43333333333333, 41.7, 39.66666666666666, 1968500.0, 0.0),
        3729 => us_state_plane_lcc_ftus(3729, "NAD 1983 NSRS2007 StatePlane Ohio South FIPS 3402 Ft US", Datum::NAD83_NSRS2007, -82.5, 38.73333333333333, 40.03333333333333, 38.0, 1968500.0, 0.0),
        3730 => us_state_plane_tm_ftus(3730, "NAD 1983 NSRS2007 StatePlane Wyoming East FIPS 4901 Ft US", Datum::NAD83_NSRS2007, -105.1666666666667, 40.5, 0.9999375, 656166.6666666665, 0.0),
        3731 => us_state_plane_tm_ftus(3731, "NAD 1983 NSRS2007 StatePlane Wyoming E Central FIPS 4902 Ft US", Datum::NAD83_NSRS2007, -107.3333333333333, 40.5, 0.9999375, 1312333.333333333, 328083.3333333333),
        3732 => us_state_plane_tm_ftus(3732, "NAD 1983 NSRS2007 StatePlane Wyoming W Central FIPS 4903 Ft US", Datum::NAD83_NSRS2007, -108.75, 40.5, 0.9999375, 1968500.0, 0.0),
        3733 => us_state_plane_tm_ftus(3733, "NAD 1983 NSRS2007 StatePlane Wyoming West FIPS 4904 Ft US", Datum::NAD83_NSRS2007, -110.0833333333333, 40.5, 0.9999375, 2624666.666666666, 328083.3333333333),
        3734 => us_state_plane_lcc_ftus(3734, "NAD 1983 StatePlane Ohio North FIPS 3401 Feet", Datum::NAD83, -82.5, 40.43333333333333, 41.7, 39.66666666666666, 1968500.0, 0.0),
        3735 => us_state_plane_lcc_ftus(3735, "NAD 1983 StatePlane Ohio South FIPS 3402 Feet", Datum::NAD83, -82.5, 38.73333333333333, 40.03333333333333, 38.0, 1968500.0, 0.0),
        3736 => us_state_plane_tm_ftus(3736, "NAD 1983 StatePlane Wyoming East FIPS 4901 Feet", Datum::NAD83, -105.1666666666667, 40.5, 0.9999375, 656166.6666666665, 0.0),
        3737 => us_state_plane_tm_ftus(3737, "NAD 1983 StatePlane Wyoming East Central FIPS 4902 Feet", Datum::NAD83, -107.3333333333333, 40.5, 0.9999375, 1312333.333333333, 328083.3333333333),
        3738 => us_state_plane_tm_ftus(3738, "NAD 1983 StatePlane Wyoming West Central FIPS 4903 Feet", Datum::NAD83, -108.75, 40.5, 0.9999375, 1968500.0, 0.0),
        3739 => us_state_plane_tm_ftus(3739, "NAD 1983 StatePlane Wyoming West FIPS 4904 Feet", Datum::NAD83, -110.0833333333333, 40.5, 0.9999375, 2624666.666666666, 328083.3333333333),
        3740 => us_state_plane_tm(3740, "NAD 1983 HARN UTM Zone 10N", Datum::NAD83_HARN, -123.0, 0.0, 0.9996, 500000.0, 0.0),
        3741 => us_state_plane_tm(3741, "NAD 1983 HARN UTM Zone 11N", Datum::NAD83_HARN, -117.0, 0.0, 0.9996, 500000.0, 0.0),
        3742 => us_state_plane_tm(3742, "NAD 1983 HARN UTM Zone 12N", Datum::NAD83_HARN, -111.0, 0.0, 0.9996, 500000.0, 0.0),
        3743 => us_state_plane_tm(3743, "NAD 1983 HARN UTM Zone 13N", Datum::NAD83_HARN, -105.0, 0.0, 0.9996, 500000.0, 0.0),
        3744 => us_state_plane_tm(3744, "NAD 1983 HARN UTM Zone 14N", Datum::NAD83_HARN, -99.0, 0.0, 0.9996, 500000.0, 0.0),
        3745 => us_state_plane_tm(3745, "NAD 1983 HARN UTM Zone 15N", Datum::NAD83_HARN, -93.0, 0.0, 0.9996, 500000.0, 0.0),
        3746 => us_state_plane_tm(3746, "NAD 1983 HARN UTM Zone 16N", Datum::NAD83_HARN, -87.0, 0.0, 0.9996, 500000.0, 0.0),
        3747 => us_state_plane_tm(3747, "NAD 1983 HARN UTM Zone 17N", Datum::NAD83_HARN, -81.0, 0.0, 0.9996, 500000.0, 0.0),
        3748 => us_state_plane_tm(3748, "NAD 1983 HARN UTM Zone 18N", Datum::NAD83_HARN, -75.0, 0.0, 0.9996, 500000.0, 0.0),
        3749 => us_state_plane_tm(3749, "NAD 1983 HARN UTM Zone 19N", Datum::NAD83_HARN, -69.0, 0.0, 0.9996, 500000.0, 0.0),
        3750 => us_state_plane_tm(3750, "NAD 1983 HARN UTM Zone 4N", Datum::NAD83_HARN, -159.0, 0.0, 0.9996, 500000.0, 0.0),
        3751 => us_state_plane_tm(3751, "NAD 1983 HARN UTM Zone 5N", Datum::NAD83_HARN, -153.0, 0.0, 0.9996, 500000.0, 0.0),
        _ => Err(ProjectionError::UnsupportedProjection(
            format!("EPSG:{code} is not supported in 3580-3751 family helper"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_lcc(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat1: impl Into<f64>, lat2: impl Into<f64>, lat0: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat1 = lat1.into();
    let lat2 = lat2.into();
    let lat0 = lat0.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::LambertConformalConic {
                lat1,
                lat2: Some(lat2),
            })
            .with_lon0(lon0)
            .with_lat0(lat0)
            .with_false_easting(fe)
            .with_false_northing(fn_)
            .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_tm(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat0: impl Into<f64>, scale: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat0 = lat0.into();
    let scale = scale.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(fe)
                .with_false_northing(fn_)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_tm_ft(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat0: impl Into<f64>, scale: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat0 = lat0.into();
    let scale = scale.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    let international_foot = 0.3048;
    let semi_major_axis_ft = datum.ellipsoid.a / international_foot;
    let inverse_flattening = 1.0 / datum.ellipsoid.f;

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(fe)
                .with_false_northing(fn_)
                .with_ellipsoid(Ellipsoid::from_a_inv_f(
                    "GRS80 (ft)",
                    semi_major_axis_ft,
                    inverse_flattening,
                )),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_tm_ftus(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat0: impl Into<f64>, scale: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat0 = lat0.into();
    let scale = scale.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    let us_survey_foot = 1200.0 / 3937.0;
    let semi_major_axis_ft = datum.ellipsoid.a / us_survey_foot;
    let inverse_flattening = 1.0 / datum.ellipsoid.f;

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(scale)
                .with_false_easting(fe)
                .with_false_northing(fn_)
                .with_ellipsoid(Ellipsoid::from_a_inv_f(
                    "GRS80 (ftUS)",
                    semi_major_axis_ft,
                    inverse_flattening,
                )),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_lcc_ft(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat1: impl Into<f64>, lat2: impl Into<f64>, lat0: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat1 = lat1.into();
    let lat2 = lat2.into();
    let lat0 = lat0.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    let international_foot = 0.3048;
    let semi_major_axis_ft = datum.ellipsoid.a / international_foot;
    let inverse_flattening = 1.0 / datum.ellipsoid.f;

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::LambertConformalConic {
                lat1,
                lat2: Some(lat2),
            })
            .with_lon0(lon0)
            .with_lat0(lat0)
            .with_false_easting(fe)
            .with_false_northing(fn_)
            .with_ellipsoid(Ellipsoid::from_a_inv_f(
                "GRS80 (ft)",
                semi_major_axis_ft,
                inverse_flattening,
            )),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_lcc_ftus(
    code: u32, name: &str, datum: Datum,
    lon0: impl Into<f64>, lat1: impl Into<f64>, lat2: impl Into<f64>, lat0: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lon0 = lon0.into();
    let lat1 = lat1.into();
    let lat2 = lat2.into();
    let lat0 = lat0.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    let us_survey_foot = 1200.0 / 3937.0;
    let semi_major_axis_ft = datum.ellipsoid.a / us_survey_foot;
    let inverse_flattening = 1.0 / datum.ellipsoid.f;

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::LambertConformalConic {
                lat1,
                lat2: Some(lat2),
            })
            .with_lon0(lon0)
            .with_lat0(lat0)
            .with_false_easting(fe)
            .with_false_northing(fn_)
            .with_ellipsoid(Ellipsoid::from_a_inv_f(
                "GRS80 (ftUS)",
                semi_major_axis_ft,
                inverse_flattening,
            )),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn us_state_plane_omerc(
    code: u32, name: &str, datum: Datum,
    lonc: impl Into<f64>, latc: impl Into<f64>, azimuth: impl Into<f64>, scale: impl Into<f64>,
    fe: impl Into<f64>, fn_: impl Into<f64>,
) -> Result<Crs> {
    let lonc = lonc.into();
    let latc = latc.into();
    let azimuth = azimuth.into();
    let scale = scale.into();
    let fe = fe.into();
    let fn_ = fn_.into();

    Ok(Crs {
        name: format!("{name} (EPSG:{code})"),
        datum: datum.clone(),
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::HotineObliqueMercator {
                azimuth,
                rectified_grid_angle: None,
            })
                .with_lon0(lonc)
                .with_lat0(latc)
                .with_scale(scale)
                .with_false_easting(fe)
                .with_false_northing(fn_)
                .with_ellipsoid(datum.ellipsoid.clone()),
        )?,
    })
}

fn japan_plane(code: u32, zone: u32, lon0: f64, lat0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("JGD2011 / Japan Plane CS {zone} (EPSG:{code})"),
        datum: Datum::JGD2011,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(0.9999)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn japan_plane_jgd2000(code: u32, zone: u32, lon0: f64, lat0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("JGD2000 / Japan Plane CS {zone} (EPSG:{code})"),
        datum: Datum::JGD2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(lat0)
                .with_scale(0.9999)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn jgd2011_utm_crs(code: u32, zone: u8) -> Result<Crs> {
    Ok(Crs {
        name: format!("JGD2011 / UTM zone {}N (EPSG:{code})", zone),
        datum: Datum::JGD2011,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, false)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn rdn2008_utm_crs(code: u32, zone: u8) -> Result<Crs> {
    Ok(Crs {
        name: format!("RDN2008 / UTM zone {}N (N-E) (EPSG:{code})", zone),
        datum: Datum::RDN2008,
        projection: crate::projections::Projection::new(
            ProjectionParams::utm(zone, false)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn gda94_mga_variant_crs(code: u32, zone: u8) -> Result<Crs> {
    Ok(Crs {
        name: format!("GDA94 / MGA zone {zone} (EPSG:{code})"),
        datum: Datum::GDA94,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(-183.0 + 6.0 * f64::from(zone))
                .with_lat0(0.0)
                .with_scale(0.9996)
                .with_false_easting(500_000.0)
                .with_false_northing(10_000_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn cgcs2000_gk_zone_crs(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("CGCS2000 / Gauss-Kruger zone {zone} (EPSG:{code})"),
        datum: Datum::CGCS2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(f64::from(zone) * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn cgcs2000_gk_cm_crs(code: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("CGCS2000 / Gauss-Kruger CM {lon0:.0}E (EPSG:{code})"),
        datum: Datum::CGCS2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn cgcs2000_gk_3deg_zone_crs(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("CGCS2000 / 3-degree Gauss-Kruger zone {zone} (EPSG:{code})"),
        datum: Datum::CGCS2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(f64::from(zone) * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn cgcs2000_gk_3deg_cm_crs(code: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("CGCS2000 / 3-degree Gauss-Kruger CM {lon0:.0}E (EPSG:{code})"),
        datum: Datum::CGCS2000,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn new_beijing_gk_zone_crs(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("New Beijing / Gauss-Kruger zone {zone} (EPSG:{code})"),
        datum: Datum::NEW_BEIJING,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(f64::from(zone) * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn new_beijing_gk_cm_crs(code: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("New Beijing / Gauss-Kruger CM {lon0:.0}E (EPSG:{code})"),
        datum: Datum::NEW_BEIJING,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn new_beijing_gk_3deg_zone_crs(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("New Beijing / 3-degree Gauss-Kruger zone {zone} (EPSG:{code})"),
        datum: Datum::NEW_BEIJING,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(f64::from(zone) * 1_000_000.0 + 500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn new_beijing_gk_3deg_cm_crs(code: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("New Beijing / 3-degree Gauss-Kruger CM {lon0:.0}E (EPSG:{code})"),
        datum: Datum::NEW_BEIJING,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(500_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::KRASSOWSKY1940),
        )?,
    })
}

fn etrs89_nor_ntm_crs(code: u32, zone: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("ETRS89-NOR [EUREF89] / NTM zone {zone} (EPSG:{code})"),
        datum: Datum::ETRS89,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(100_000.0)
                .with_false_northing(1_000_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn south_africa_lo(code: u32, lon0: f64) -> Result<Crs> {
    Ok(Crs {
        name: format!("Cape / Lo{lon0:.0} (EPSG:{code})"),
        datum: Datum::CAPE,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::CLARKE1866),
        )?,
    })
}

fn gda2020_mga_crs(code: u32, zone: u8) -> Result<Crs> {
    Ok(Crs {
        name: format!("GDA2020 / MGA zone {zone} (EPSG:{code})"),
        datum: Datum::GDA2020,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(-183.0 + 6.0 * f64::from(zone))
                .with_lat0(0.0)
                .with_scale(0.9996)
                .with_false_easting(500_000.0)
                .with_false_northing(10_000_000.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn sweref99_local_tm(code: u32, lon0: f64) -> Result<Crs> {
    let lon0_deg = lon0.trunc() as i32;
    let lon0_min = ((lon0 - lon0.trunc()) * 60.0).round() as i32;
    Ok(Crs {
        name: format!("SWEREF99 {:02} {:02} (EPSG:{code})", lon0_deg, lon0_min),
        datum: Datum::ETRS89,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(1.0)
                .with_false_easting(150_000.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}

fn poland_cs2000(code: u32, lon0: f64, false_easting: f64) -> Result<Crs> {
    let zone = (lon0 / 3.0).round() as i32;
    Ok(Crs {
        name: format!("ETRS89 / Poland CS2000 zone {zone} (EPSG:{code})"),
        datum: Datum::ETRS89,
        projection: crate::projections::Projection::new(
            ProjectionParams::new(ProjectionKind::TransverseMercator)
                .with_lon0(lon0)
                .with_lat0(0.0)
                .with_scale(0.999_923)
                .with_false_easting(false_easting)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::GRS80),
        )?,
    })
}
