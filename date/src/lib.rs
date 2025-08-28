// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }

//TODO: Write tests for regex evals for each type of date

pub mod date_utils {
    use std::str::FromStr;
    use regex::{Captures, Regex};

    #[derive(PartialEq, Eq)]
    pub enum RemindMeDateTypes {
        Invalid,
        ThreeLetterMonth { d: u32, mon: u32, y: i32 },
        SpecifiedTime{ h: u32, min: u32 },
        AddedTime{ y: i32, mon: u32, d: u32, h: u32, min: u32 }, // TODO: Should be Option<T> for each number
    }

    pub fn parse_date(str: &str) -> RemindMeDateTypes {

        // TODO: Year first date, whatever, do later
        // TODO: Refactor function to return result (Replace invalid with error)
        if str.is_empty() {
            return RemindMeDateTypes::Invalid;
        }

        // Three letter month 
        // TODO don't use group tags, might be faster?
        // TODO the regex should be unit tested
        let three_letter_month_reg = Regex::new(r"^(?<day>\d{1,2})(?<month>\w{3})(?<year>\d{2,4})$").unwrap();
        if let Some(three_letter_month_caps) = three_letter_month_reg.captures(str) {
            // three_letter_month reg should only match is a day, 3 letter word and year are parseable
            let day: u32 = regex_cap2num(&three_letter_month_caps, "day").unwrap();

            // Return invalid if a 3 letter month is not provided
            let Some(month_num) = month_to_number(three_letter_month_caps.name("month").unwrap().as_str()) else {
                return RemindMeDateTypes::Invalid;
            };
            let year: i32 = regex_cap2num(&three_letter_month_caps, "year").unwrap();

            return RemindMeDateTypes::ThreeLetterMonth{ d: day, mon: month_num, y: year};
        }

        // Specified time
        let specified_time_reg = Regex::new(r"^(?<hour>([0-2])([0-3]))(?<minute>([0-5])(\d))$").unwrap();
        if let Some(specified_time_caps) = specified_time_reg.captures(str) {
            // specified_time_reg.is_match will always return parseable numbers in this format
            let hour = regex_cap2num(&specified_time_caps, "hour").unwrap();
            let minute = regex_cap2num(&specified_time_caps, "minute").unwrap();
            return RemindMeDateTypes::SpecifiedTime{ h: hour, min: minute};
        }

        // Is added time
        // TODO regex definitely needs unit testing, thank god for the debugger and vim %
        let added_time_reg = Regex::new(r"^((?<year>\d+)[y])?((?<month>\d+)[M])?((?<day>\d+)[d])?((?<hour>\d+)[h])?((?<minute>\d+)[m])?$").unwrap();
        if let Some(added_time_caps) = added_time_reg.captures(str) {
            // TODO when numbers are replaced with Option<T> use .ok() instead of unwrap_or_default()
            let year: i32 = regex_cap2num(&added_time_caps, "year").unwrap_or_default();
            let month: u32 = regex_cap2num(&added_time_caps, "month").unwrap_or_default();
            let day: u32 = regex_cap2num(&added_time_caps, "day").unwrap_or_default();
            let hour: u32 = regex_cap2num(&added_time_caps, "hour").unwrap_or_default();
            let minute: u32 = regex_cap2num(&added_time_caps, "minute").unwrap_or_default();

            return RemindMeDateTypes::AddedTime{ y: year, mon: month, d: day, h: hour, min: minute};
        }

        return RemindMeDateTypes::Invalid;
    }

    fn month_to_number(mon: &str) -> Option<u32> {
        match mon.to_ascii_lowercase().as_str() {
            "jan" => Some(0),
            "feb" => Some(1),
            "mar" => Some(2),
            "apr" => Some(3),
            "may" => Some(4),
            "jun" => Some(5),
            "jul" => Some(6),
            "aug" => Some(7),
            "sep" => Some(8),
            "oct" => Some(9),
            "nov" => Some(10),
            "dec" => Some(11),
            _ => None,
        }
    }

    fn regex_cap2num<T: FromStr>(caps: &Captures, name: &str) -> Result<T, <T as FromStr>::Err> {
        return str::parse::<T>(caps.name(name).map_or("", |m| m.as_str()));
    }
}