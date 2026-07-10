use crate::amount::Amount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationProfile {
    pub code: &'static str,
    pub tier: &'static str,
    pub day_floor: u64,
    pub day_ceiling: u64,
    pub priority_capacity: u64,
    pub standard_capacity: u64,
    pub user_ceiling: u64,
    pub reserve_buffer_bps: u32,
    pub release_rate_bps: u32,
    pub note: &'static str,
}

impl CalibrationProfile {
    pub fn priority_amount(self) -> Amount {
        Amount::from(self.priority_capacity)
    }

    pub fn standard_amount(self) -> Amount {
        Amount::from(self.standard_capacity)
    }

    pub fn user_ceiling_amount(self) -> Amount {
        Amount::from(self.user_ceiling)
    }

    pub fn covers_day(self, day: u64) -> bool {
        day >= self.day_floor && day <= self.day_ceiling
    }

    pub fn matches_tier(self, tier: &str) -> bool {
        self.tier.eq_ignore_ascii_case(tier)
    }

    pub fn effective_priority_after_buffer(self) -> Amount {
        let retained = self
            .priority_capacity
            .saturating_mul((10_000 - self.reserve_buffer_bps) as u64)
            / 10_000;
        Amount::from(retained)
    }

    pub fn release_amount(self) -> Amount {
        let released = self
            .standard_capacity
            .saturating_mul(self.release_rate_bps as u64)
            / 10_000;
        Amount::from(released)
    }
}

macro_rules! profile {
    (
        $code:literal,
        $tier:literal,
        $day_floor:expr,
        $day_ceiling:expr,
        $priority_capacity:expr,
        $standard_capacity:expr,
        $user_ceiling:expr,
        $reserve_buffer_bps:expr,
        $release_rate_bps:expr,
        $note:literal
    ) => {
        CalibrationProfile {
            code: $code,
            tier: $tier,
            day_floor: $day_floor,
            day_ceiling: $day_ceiling,
            priority_capacity: $priority_capacity,
            standard_capacity: $standard_capacity,
            user_ceiling: $user_ceiling,
            reserve_buffer_bps: $reserve_buffer_bps,
            release_rate_bps: $release_rate_bps,
            note: $note,
        }
    };
}

pub const PROFILE_CATALOG: &[CalibrationProfile] = &[
    profile!(
        "CRN-001",
        "retail",
        0,
        2,
        420,
        980,
        260,
        700,
        2400,
        "low traffic retail opening window"
    ),
    profile!(
        "CRN-002",
        "vip",
        0,
        2,
        900,
        1420,
        620,
        620,
        2850,
        "vip opening window with elevated turnover"
    ),
    profile!(
        "CRN-003",
        "institutional",
        0,
        2,
        1480,
        2200,
        1040,
        560,
        3300,
        "institutional launch allocation"
    ),
    profile!(
        "CRN-004",
        "stabilizer",
        0,
        2,
        1800,
        2600,
        1220,
        520,
        3600,
        "stabilizer lane for launch balancing"
    ),
    profile!(
        "CRN-005",
        "retail",
        3,
        5,
        460,
        1020,
        280,
        700,
        2425,
        "retail window after first unlock"
    ),
    profile!(
        "CRN-006",
        "vip",
        3,
        5,
        960,
        1500,
        660,
        620,
        2875,
        "vip window after first unlock"
    ),
    profile!(
        "CRN-007",
        "institutional",
        3,
        5,
        1540,
        2280,
        1080,
        560,
        3325,
        "institutional window after first unlock"
    ),
    profile!(
        "CRN-008",
        "stabilizer",
        3,
        5,
        1860,
        2680,
        1260,
        520,
        3625,
        "stabilizer window after first unlock"
    ),
    profile!(
        "CRN-009",
        "retail",
        6,
        8,
        500,
        1060,
        300,
        690,
        2450,
        "retail steady state band"
    ),
    profile!(
        "CRN-010",
        "vip",
        6,
        8,
        1020,
        1580,
        700,
        610,
        2900,
        "vip steady state band"
    ),
    profile!(
        "CRN-011",
        "institutional",
        6,
        8,
        1600,
        2360,
        1120,
        550,
        3350,
        "institutional steady state band"
    ),
    profile!(
        "CRN-012",
        "stabilizer",
        6,
        8,
        1920,
        2760,
        1300,
        510,
        3650,
        "stabilizer steady state band"
    ),
    profile!(
        "CRN-013",
        "retail",
        9,
        11,
        540,
        1100,
        320,
        690,
        2475,
        "retail weekend settlement band"
    ),
    profile!(
        "CRN-014",
        "vip",
        9,
        11,
        1080,
        1660,
        740,
        610,
        2925,
        "vip weekend settlement band"
    ),
    profile!(
        "CRN-015",
        "institutional",
        9,
        11,
        1660,
        2440,
        1160,
        550,
        3375,
        "institutional weekend settlement band"
    ),
    profile!(
        "CRN-016",
        "stabilizer",
        9,
        11,
        1980,
        2840,
        1340,
        510,
        3675,
        "stabilizer weekend settlement band"
    ),
    profile!(
        "CRN-017",
        "retail",
        12,
        14,
        580,
        1140,
        340,
        680,
        2500,
        "retail mid cycle band"
    ),
    profile!(
        "CRN-018",
        "vip",
        12,
        14,
        1140,
        1740,
        780,
        600,
        2950,
        "vip mid cycle band"
    ),
    profile!(
        "CRN-019",
        "institutional",
        12,
        14,
        1720,
        2520,
        1200,
        540,
        3400,
        "institutional mid cycle band"
    ),
    profile!(
        "CRN-020",
        "stabilizer",
        12,
        14,
        2040,
        2920,
        1380,
        500,
        3700,
        "stabilizer mid cycle band"
    ),
    profile!(
        "CRN-021",
        "retail",
        15,
        17,
        620,
        1180,
        360,
        680,
        2525,
        "retail cycle expansion"
    ),
    profile!(
        "CRN-022",
        "vip",
        15,
        17,
        1200,
        1820,
        820,
        600,
        2975,
        "vip cycle expansion"
    ),
    profile!(
        "CRN-023",
        "institutional",
        15,
        17,
        1780,
        2600,
        1240,
        540,
        3425,
        "institutional cycle expansion"
    ),
    profile!(
        "CRN-024",
        "stabilizer",
        15,
        17,
        2100,
        3000,
        1420,
        500,
        3725,
        "stabilizer cycle expansion"
    ),
    profile!(
        "CRN-025",
        "retail",
        18,
        20,
        660,
        1220,
        380,
        670,
        2550,
        "retail late cycle band"
    ),
    profile!(
        "CRN-026",
        "vip",
        18,
        20,
        1260,
        1900,
        860,
        590,
        3000,
        "vip late cycle band"
    ),
    profile!(
        "CRN-027",
        "institutional",
        18,
        20,
        1840,
        2680,
        1280,
        530,
        3450,
        "institutional late cycle band"
    ),
    profile!(
        "CRN-028",
        "stabilizer",
        18,
        20,
        2160,
        3080,
        1460,
        490,
        3750,
        "stabilizer late cycle band"
    ),
    profile!(
        "CRN-029",
        "retail",
        21,
        23,
        700,
        1260,
        400,
        670,
        2575,
        "retail pre close band"
    ),
    profile!(
        "CRN-030",
        "vip",
        21,
        23,
        1320,
        1980,
        900,
        590,
        3025,
        "vip pre close band"
    ),
    profile!(
        "CRN-031",
        "institutional",
        21,
        23,
        1900,
        2760,
        1320,
        530,
        3475,
        "institutional pre close band"
    ),
    profile!(
        "CRN-032",
        "stabilizer",
        21,
        23,
        2220,
        3160,
        1500,
        490,
        3775,
        "stabilizer pre close band"
    ),
    profile!(
        "CRN-033",
        "retail",
        24,
        26,
        740,
        1300,
        420,
        660,
        2600,
        "retail close preparation"
    ),
    profile!(
        "CRN-034",
        "vip",
        24,
        26,
        1380,
        2060,
        940,
        580,
        3050,
        "vip close preparation"
    ),
    profile!(
        "CRN-035",
        "institutional",
        24,
        26,
        1960,
        2840,
        1360,
        520,
        3500,
        "institutional close preparation"
    ),
    profile!(
        "CRN-036",
        "stabilizer",
        24,
        26,
        2280,
        3240,
        1540,
        480,
        3800,
        "stabilizer close preparation"
    ),
    profile!(
        "CRN-037",
        "retail",
        27,
        29,
        780,
        1340,
        440,
        660,
        2625,
        "retail monthly close"
    ),
    profile!(
        "CRN-038",
        "vip",
        27,
        29,
        1440,
        2140,
        980,
        580,
        3075,
        "vip monthly close"
    ),
    profile!(
        "CRN-039",
        "institutional",
        27,
        29,
        2020,
        2920,
        1400,
        520,
        3525,
        "institutional monthly close"
    ),
    profile!(
        "CRN-040",
        "stabilizer",
        27,
        29,
        2340,
        3320,
        1580,
        480,
        3825,
        "stabilizer monthly close"
    ),
    profile!(
        "CRN-041",
        "retail",
        30,
        32,
        760,
        1320,
        430,
        665,
        2610,
        "retail new cycle reset"
    ),
    profile!(
        "CRN-042",
        "vip",
        30,
        32,
        1400,
        2100,
        960,
        585,
        3060,
        "vip new cycle reset"
    ),
    profile!(
        "CRN-043",
        "institutional",
        30,
        32,
        1980,
        2880,
        1380,
        525,
        3510,
        "institutional new cycle reset"
    ),
    profile!(
        "CRN-044",
        "stabilizer",
        30,
        32,
        2300,
        3280,
        1560,
        485,
        3810,
        "stabilizer new cycle reset"
    ),
    profile!(
        "CRN-045",
        "retail",
        33,
        35,
        720,
        1280,
        410,
        675,
        2585,
        "retail cool down band"
    ),
    profile!(
        "CRN-046",
        "vip",
        33,
        35,
        1340,
        2020,
        920,
        595,
        3035,
        "vip cool down band"
    ),
    profile!(
        "CRN-047",
        "institutional",
        33,
        35,
        1920,
        2800,
        1340,
        535,
        3485,
        "institutional cool down band"
    ),
    profile!(
        "CRN-048",
        "stabilizer",
        33,
        35,
        2240,
        3200,
        1520,
        495,
        3785,
        "stabilizer cool down band"
    ),
    profile!(
        "CRN-049",
        "retail",
        36,
        38,
        680,
        1240,
        390,
        685,
        2560,
        "retail compression band"
    ),
    profile!(
        "CRN-050",
        "vip",
        36,
        38,
        1280,
        1940,
        880,
        605,
        3010,
        "vip compression band"
    ),
    profile!(
        "CRN-051",
        "institutional",
        36,
        38,
        1860,
        2720,
        1300,
        545,
        3460,
        "institutional compression band"
    ),
    profile!(
        "CRN-052",
        "stabilizer",
        36,
        38,
        2180,
        3120,
        1480,
        505,
        3760,
        "stabilizer compression band"
    ),
    profile!(
        "CRN-053",
        "retail",
        39,
        41,
        640,
        1200,
        370,
        695,
        2535,
        "retail reserve rebuild"
    ),
    profile!(
        "CRN-054",
        "vip",
        39,
        41,
        1220,
        1860,
        840,
        615,
        2985,
        "vip reserve rebuild"
    ),
    profile!(
        "CRN-055",
        "institutional",
        39,
        41,
        1800,
        2640,
        1260,
        555,
        3435,
        "institutional reserve rebuild"
    ),
    profile!(
        "CRN-056",
        "stabilizer",
        39,
        41,
        2120,
        3040,
        1440,
        515,
        3735,
        "stabilizer reserve rebuild"
    ),
    profile!(
        "CRN-057",
        "retail",
        42,
        44,
        600,
        1160,
        350,
        705,
        2510,
        "retail conservative band"
    ),
    profile!(
        "CRN-058",
        "vip",
        42,
        44,
        1160,
        1780,
        800,
        625,
        2960,
        "vip conservative band"
    ),
    profile!(
        "CRN-059",
        "institutional",
        42,
        44,
        1740,
        2560,
        1220,
        565,
        3410,
        "institutional conservative band"
    ),
    profile!(
        "CRN-060",
        "stabilizer",
        42,
        44,
        2060,
        2960,
        1400,
        525,
        3710,
        "stabilizer conservative band"
    ),
    profile!(
        "CRN-061",
        "retail",
        45,
        47,
        560,
        1120,
        330,
        715,
        2485,
        "retail drawdown guard"
    ),
    profile!(
        "CRN-062",
        "vip",
        45,
        47,
        1100,
        1700,
        760,
        635,
        2935,
        "vip drawdown guard"
    ),
    profile!(
        "CRN-063",
        "institutional",
        45,
        47,
        1680,
        2480,
        1180,
        575,
        3385,
        "institutional drawdown guard"
    ),
    profile!(
        "CRN-064",
        "stabilizer",
        45,
        47,
        2000,
        2880,
        1360,
        535,
        3685,
        "stabilizer drawdown guard"
    ),
    profile!(
        "CRN-065",
        "retail",
        48,
        50,
        520,
        1080,
        310,
        725,
        2460,
        "retail high buffer band"
    ),
    profile!(
        "CRN-066",
        "vip",
        48,
        50,
        1040,
        1620,
        720,
        645,
        2910,
        "vip high buffer band"
    ),
    profile!(
        "CRN-067",
        "institutional",
        48,
        50,
        1620,
        2400,
        1140,
        585,
        3360,
        "institutional high buffer band"
    ),
    profile!(
        "CRN-068",
        "stabilizer",
        48,
        50,
        1940,
        2800,
        1320,
        545,
        3660,
        "stabilizer high buffer band"
    ),
    profile!(
        "CRN-069",
        "retail",
        51,
        53,
        480,
        1040,
        290,
        735,
        2435,
        "retail low release band"
    ),
    profile!(
        "CRN-070",
        "vip",
        51,
        53,
        980,
        1540,
        680,
        655,
        2885,
        "vip low release band"
    ),
    profile!(
        "CRN-071",
        "institutional",
        51,
        53,
        1560,
        2320,
        1100,
        595,
        3335,
        "institutional low release band"
    ),
    profile!(
        "CRN-072",
        "stabilizer",
        51,
        53,
        1880,
        2720,
        1280,
        555,
        3635,
        "stabilizer low release band"
    ),
    profile!(
        "CRN-073",
        "retail",
        54,
        56,
        440,
        1000,
        270,
        745,
        2410,
        "retail stress entry"
    ),
    profile!(
        "CRN-074",
        "vip",
        54,
        56,
        920,
        1460,
        640,
        665,
        2860,
        "vip stress entry"
    ),
    profile!(
        "CRN-075",
        "institutional",
        54,
        56,
        1500,
        2240,
        1060,
        605,
        3310,
        "institutional stress entry"
    ),
    profile!(
        "CRN-076",
        "stabilizer",
        54,
        56,
        1820,
        2640,
        1240,
        565,
        3610,
        "stabilizer stress entry"
    ),
    profile!(
        "CRN-077",
        "retail",
        57,
        59,
        400,
        960,
        250,
        755,
        2385,
        "retail stress close"
    ),
    profile!(
        "CRN-078",
        "vip",
        57,
        59,
        860,
        1380,
        600,
        675,
        2835,
        "vip stress close"
    ),
    profile!(
        "CRN-079",
        "institutional",
        57,
        59,
        1440,
        2160,
        1020,
        615,
        3285,
        "institutional stress close"
    ),
    profile!(
        "CRN-080",
        "stabilizer",
        57,
        59,
        1760,
        2560,
        1200,
        575,
        3585,
        "stabilizer stress close"
    ),
];

pub fn profile_by_code(code: &str) -> Option<CalibrationProfile> {
    PROFILE_CATALOG
        .iter()
        .copied()
        .find(|profile| profile.code.eq_ignore_ascii_case(code))
}

pub fn profiles_for_tier(tier: &str) -> Vec<CalibrationProfile> {
    PROFILE_CATALOG
        .iter()
        .copied()
        .filter(|profile| profile.matches_tier(tier))
        .collect()
}

pub fn profiles_for_day(day: u64) -> Vec<CalibrationProfile> {
    PROFILE_CATALOG
        .iter()
        .copied()
        .filter(|profile| profile.covers_day(day))
        .collect()
}

pub fn recommended_profile(tier: &str, day: u64) -> Option<CalibrationProfile> {
    PROFILE_CATALOG
        .iter()
        .copied()
        .filter(|profile| profile.matches_tier(tier) && profile.covers_day(day))
        .max_by_key(|profile| {
            (
                profile.priority_capacity,
                profile.standard_capacity,
                10_000_u32.saturating_sub(profile.reserve_buffer_bps),
            )
        })
}

pub fn total_priority_for_day(day: u64) -> Amount {
    let total = PROFILE_CATALOG
        .iter()
        .filter(|profile| profile.covers_day(day))
        .fold(0_u64, |acc, profile| {
            acc.saturating_add(profile.priority_capacity)
        });
    Amount::from(total)
}

pub fn total_standard_for_day(day: u64) -> Amount {
    let total = PROFILE_CATALOG
        .iter()
        .filter(|profile| profile.covers_day(day))
        .fold(0_u64, |acc, profile| {
            acc.saturating_add(profile.standard_capacity)
        });
    Amount::from(total)
}
