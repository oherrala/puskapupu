use std::str::FromStr;

use chumsky::prelude::*;
use chumsky::Parser;

#[derive(Debug)]
pub struct DxEntry {
    pub reporter: String,
    pub frequency: f32,
    pub dx: String,
    pub cqgma_identifier: Option<(Activity, Source)>,
    pub info: String,
    pub timestamp: String,
}

impl FromStr for DxEntry {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        dxspider_parser().parse(s).map_err(|_| ())
    }
}

// use this codes:                 origin of spots for your info
// -------------------------------------------------------------------
// x01 = Flora & Fauna         |  d = DX Cluster     s = SOTAwatch RSS
// x02 = Islands               |  f = smartWWFF      t = RRT
// X03 = Castles               |  g = GMAwatch       u = UDXlog
// x04 = SOTA                  |  m = smartGMA       v = VK Spots
// X05 = GMA                   |  r = RBN            w = WWFFwatch
// X06 = Lighthouses           |                     x = SMS
// X07 = RDA                   |
// x08 = AGCW                  |

#[derive(Debug)]
pub enum Activity {
    /// Flora & Fauna
    Wwff,
    /// Islands on the Air
    Iota,
    /// Castles on the Air
    Cota,
    /// Summits on the Air
    Sota,
    /// Global Mountain Activity
    Gma,
    /// Lighthouses
    Lighthouses,
    Rda,
    Agcw,
}

impl FromStr for Activity {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "01" => Ok(Activity::Wwff),
            "02" => Ok(Activity::Iota),
            "03" => Ok(Activity::Cota),
            "04" => Ok(Activity::Sota),
            "05" => Ok(Activity::Gma),
            "06" => Ok(Activity::Lighthouses),
            "07" => Ok(Activity::Rda),
            "08" => Ok(Activity::Agcw),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Source {
    DxCluster,
    SmartWwff,
    GmaWatch,
    SmartGma,
    Rbn,
    SotaWatchRss,
    Rrt,
    UdxLog,
    VkSpots,
    WwffWatch,
    Sms,
}

impl FromStr for Source {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "d" => Ok(Source::DxCluster),
            "f" => Ok(Source::SmartWwff),
            "g" => Ok(Source::GmaWatch),
            "m" => Ok(Source::SmartGma),
            "r" => Ok(Source::Rbn),
            "s" => Ok(Source::SotaWatchRss),
            "t" => Ok(Source::Rrt),
            "u" => Ok(Source::UdxLog),
            "v" => Ok(Source::VkSpots),
            "w" => Ok(Source::WwffWatch),
            "x" => Ok(Source::Sms),
            _ => Err(()),
        }
    }
}

fn dxspider_parser() -> impl Parser<char, DxEntry, Error = Simple<char>> {
    let callsign = filter(|c: &char| c.is_ascii() && *c != ':' && *c != ' ')
        .repeated()
        .collect();

    let frequency = filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(3)
        .collect()
        .map(|s: String| s.parse().unwrap());

    let cqgma_identifier = {
        let activity = filter(|c: &char| c.is_ascii_digit())
            .repeated()
            .exactly(2)
            .collect()
            .try_map(|s: String, span| {
                s.parse()
                    .map_err(|_| Simple::custom(span, "invalid activity"))
            });

        let source = filter(|c: &char| c.is_alphabetic())
            .repeated()
            .exactly(1)
            .collect()
            .try_map(|s: String, span| {
                s.parse()
                    .map_err(|_| Simple::custom(span, "invalid source"))
            });

        just('x')
            .ignored()
            .then(activity)
            .then(source)
            .map(|(((), activity), source)| (activity, source))
    };

    let info = filter(|c: &char| c.is_ascii())
        .repeated()
        .at_most(26)
        .collect()
        .map(|s: String| s.trim().to_string());

    let timestamp = text::digits(10).then_ignore(just("Z"));

    just("DX de")
        .ignored()
        .then(callsign.padded())
        .then_ignore(just(':').or_not())
        .then(frequency.padded())
        .then(callsign.padded())
        .then(cqgma_identifier.padded().or_not())
        .then(info.padded())
        .then(timestamp.padded())
        .map(|value| {
            let (value, timestamp) = value;
            let (value, info) = value;
            let (value, cqgma_identifier) = value;
            let (value, dx) = value;
            let (value, frequency) = value;
            let ((), reporter) = value;

            DxEntry {
                reporter,
                frequency,
                dx,
                cqgma_identifier,
                info,
                timestamp,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::dxspider_parser;
    use chumsky::Parser;

    const TEST: &[&str] = &[
        "DX de HB9BIN:    14044.0  HB9BIN/P     x04s HB/BL-001                 1049Z",
        "DX de KG5ED:     14074.1  VK3ACE       x02d ccc vk3* iota oc-001      1051Z",
        "DX de DL1CR:      3567.0  DL1CR/P      x04s DM/NS-107                 1052Z",
        "DX de YL2AG:     10119.0  DL2DXA/P     x01d dlff-0794.da/sx-398       1053Z",
        "DX de IZ1TNA:    14248.0  PD0RWL/P     x01d paff-0237                 1056Z",
        "DX de DL1CR:     14333.0  DL1CR/P      x04s DM/NS-107                 1057Z",
        "DX de RBNHOLE    10117.0  HB9CBR/P     x04s HB/VD-029                 1058Z",
        "DX de OK4KOP:     7067.0  OK4KOP/P     x04s OK/PA-012                 1058Z",
        "DX de HB3XXX:   145525.0  HB3XXX/P     x04s HB/GL-047                 1059Z",
        "DX de HB9BIN:    10124.0  HB9BIN/P     x04s HB/BL-001                 1101Z",
        "DX de DL3NM:     10124.0  HB9BIN/P     x01d hb/bl-001 es hbff-0212    1103Z",
        "DX de HB9BIN:     7174.0  HB9BIN/P     x04s HB/BL-001                 1105Z",
        "DX de W9SSN:    146500.0  W9SSN        x04s F/AM-661                  1106Z",
        "DX de IW6OMM:    50313.0  VK8AW        x02d ccc vk8* iota oc-001      1106Z",
        "DX de HB9BIN:     7174.0  HB9BIN/P     x05g HBFF-0212                 1106Z",
        "DX de W9SSN:    145500.0  F/W9SSN/P    x04s F/AM-661                  1107Z",
        "DX de HB3XXX:   145550.0  HB3XXX/P     x04s HB/GL-047                 1110Z",
        "DX de OK2PYA:    14044.0  OK2PYA/P     x01f OKFF-2200                 1110Z",
        "DX de EA5DD:      7034.5  EA5DD        x04s EA5/AT-048                1117Z",
        "DX de APRS2SO   145475.0  HB9CYX/P     x04s HB/SO-016                 1118Z",
        "DX de HB9EVF:     5354.0  HB9EVF/P     x04s HB/GL-047                 1119Z",
        "DX de M1HAX:    145500.0  M1HAX/P      x04s G/WB-021                  1120Z",
        "DX de IW3AGO:    14297.0  IS0/IW3AGO/P x04s IS0/IS-155                1120Z",
        "DX de G3VQO:     14297.0  IS0/IW3AGO/P x05g IS0/IS-155                1123Z",
        "DX de EA5DD:      7092.0  EA5DD        x04s EA5/AT-048                1125Z",
        "DX de OK4KOP:    14305.0  OK4KOP/P     x04s OK/PA-012                 1125Z",
        "DX de HB9CBR:    18087.0  HB9CBR/P     x04s HB/VD-029                 1131Z",
        "DX de HB9EVF:    10123.0  HB9EVF/P     x04s HB/GL-047                 1131Z",
        "DX de G3VQO:     10123.0  HB9EVF/P     x05g HB/GL-047                 1131Z",
        "DX de HB9IIO:    21058.0  HB9IIO/P     x04s HB/VD-044                 1137Z",
        "DX de EA5DD:     14063.0  EA5DD        x04s EA5/AT-048                1139Z",
        "DX de IN3ENN:    18115.0  IS0/IN3ENN/P x04s IS0/IS-155                1142Z",
        "DX de RBNHOLE    18087.1  HB9CBR/P     x04s HB/VD-029                 1143Z",
        "DX de M0WCW:    145425.0  M0WCW        x04s G/LD-052                  1143Z",
        "DX de HB9IIO:    28058.0  HB9IIO/P     x04s HB/VD-044                 1146Z",
        "DX de OH2NOS:     3644.0  OH2NOS/P     x01f OHFF-1419 New one!        1146Z",
        "DX de VK2IO:     14044.0  VK2IO        x01v VKFF-1912                 1147Z",
        "DX de F4IOQ:      7090.0  F4IOQ/P      x04s FL/NO-062                 1149Z",
        "DX de IN3ENN:    21283.0  IS0/IN3ENN/P x04s IS0/IS-155                1149Z",
        "DX de HB9CBR:    21056.0  HB9CBR/P     x04s HB/VD-029                 1150Z",
        "DX de OK2PYA:     7024.5  OK2PYA/P     x01f OKFF-2200                 1151Z",
        "DX de HB9IIO:    24905.0  HB9IIO/P     x04s HB/VD-044                 1152Z",
        "DX de JA1JXT:    10136.0  VP8KCC       x02d ft8 sa-002 op vp8lp       1152Z",
        "DX de RBNHOLE    14063.0  EA5DD        x04s EA5/AT-048                1153Z",
        "DX de HB9EVF:   145525.0  HB9EVF/P     x04s HB/GL-047                 1154Z",
        "DX de HB3XXX:    21289.0  HB3XXX/P     x04s HB/GL-047                 1154Z",
        "DX de IN3ENN:     7062.0  IS0/IN3ENN/P x04s IS0/IS-155                1155Z",
        "DX de OE3IPU:    14290.0  OE3WHU/P     x04s OE/NO-080                 1155Z",
        "DX de OH2NOS:     3544.0  OH2NOS/P     x01f OHFF-1419 New one!        1156Z",
        "DX de WC1N:      14043.0  WC1N         x01f KFF-5750                  1249Z",
        "DX de RBNHOLE    21063.0  EA5DD        x04s EA5/AT-048                1250Z",
        "DX de OK1VEI:    14236.0  OH2NOS/P     x01d ohff-1419                 1250Z",
        "DX de RBNHOLE    21061.0  SQ9OZM/P     x04s SP/BS-003                 1254Z",
        "DX de OK1VEI:    14244.0  OK7DA/P      x01d okff-0212                 1254Z",
        "DX de OH2NOS:    14046.0  OH2NOS/P     x01f OHFF-1419 New one!        1254Z",
        "DX de KQ6QB:     14343.0  KQ6QB        x01f KFF-2553                  1301Z",
        "DX de KQ6QB:     14327.0  KQ6QB        x01f KFF-3603                  1302Z",
        "DX de F4JCF:     14285.0  F4JCF/P      x04s F/AM-352                  1304Z",
        "DX de OH2NOS:    28044.0  OH2NOS/P     x01f OHFF-1419 New one!        1304Z",
        "DX de OK1VEI:     7088.0  OK2APY/P     x01d okff-3102 gma ol/jm-120   1306Z",
        "DX de EA5DD:      7091.0  EA5DD        x04s EA5/AT-048                1310Z",
        "DX de SQ9OZM:    28360.0  SQ9OZM/P     x04s SP/BS-003                 1311Z",
        "DX de F4JCF:     21285.0  F4JCF/P      x04s F/AM-352                  1317Z",
        "DX de F4JCF:     28360.0  F4JCF/P      x04s F/AM-352                  1318Z",
        "DX de F4IOQ:    145500.0  F4IOQ/P      x04s FL/NO-030                 1319Z",
        "DX de F4JCF:     18130.0  F4JCF/P      x04s F/AM-352                  1320Z",
        "DX de SQ9OZM:    21303.0  SQ9OZM/P     x04s SP/BS-003                 1320Z",
        "DX de YO6CFB:    10121.0  DL2DXA/P     x06d ok da/sx-614 wca dl-01160 1321Z",
        "DX de F4JCF:     18125.0  F4JCF/P      x04s F/AM-352                  1322Z",
        "DX de F4JCF:      7085.0  F4JCF/P      x04s F/AM-352                  1326Z",
        "DX de RBNHOLE    14058.5  SQ9OZM/P     x04s SP/BS-003                 1327Z",
        "DX de RBNHOLE    14055.0  LA9PJA/P     x04s LA/HM-129                 1327Z",
        "DX de EA5DD:     10126.0  EA5DD        x04s EA5/AT-048                1328Z",
        "DX de JA4GXS:    50220.0  JA4GXS/6     x02d cw as-023 amami o         1328Z",
        "DX de SMS:       14275.0  CT7/M0NJH/P  x04s CT/DL-004                 1330Z",
        "DX de W9SSN:    145500.0  W9SSN        x04s F/AM-662                  1331Z",
        "DX de RBNHOLE    10126.0  EA5DD        x04s EA5/AT-048                1332Z",
        "DX de ON4AVT:     7143.0  OT8S         bca on-2672                    0657Z JO10",
        "DX de VK1AO:      7150.0  VK2IO        x01v VKFF-2511                 0708Z QF67",
        "DX de WX1S:      10110.3  WX1S         x04s W1/HA-203                 1924Z FN43",
        "DX de WX1S:       7031.3  WX1S         x04s W1/HA-203                 1928Z FN43",
        "DX de WX1S:      18095.0  WX1S         x04s W1/HA-203                 1936Z FN43",
        "DX de DL4MFM:   145425.0  DM7N         x05g DM/NS-001 TEST!           1411Z JO42",
        "DX de VK1AO:      7144.0  VK2IO        x01v VKFF-1267                 0753Z QF67",
        "DX de DF1WR:      7144.0  OK/DF9PE/P   x06d wca -ok-00588 --  ol-073  0708Z",
        "DX de IW2OEV:     7144.0  OK/DF9PE/P   x06d wca ok-00588 cca_ok ol-07 0711Z",
        "DX de ON3UA:     14268.0  OS8D/P       x05g ON-00859  ON-00859 ON-008 0712Z",
        "DX de IW3AGO:    14288.0  IS0/IW3AGO/P x04s IS0/IS-003                1145Z",
        "DX de SMS:       14260.0  CT7/M0NJH/P  x04s CT/TM-022                 0850Z",
        "DX de SMS:       28053.0  DU3/AJ6CL/P  x04s DU3/ZA-057                0307Z",
    ];

    #[test]
    fn test_dxentry_parser() {
        let parser = dxspider_parser();

        for line in TEST {
            let entry = parser.parse(*line).unwrap();
            dbg!(entry);
        }
    }
}
