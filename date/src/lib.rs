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
    use regex::{Captures, Regex};

    #[derive(PartialEq, Eq)]
    pub enum RemindMeDateTypes {
        Invalid,
        ThreeLetterMonth { d: u32, mon: u32, y: i32 },
        SpecifiedTime{ h: u32, min: u32 },
        AddedTime{ y: i32, mon: u32, d: u32, h: u32, min: u32 },
    }

    pub fn parse_date(str: &str) -> RemindMeDateTypes {

        // TODO: Year first date, whatever, do later
        // TODO: Refactor function to return result
        if str.is_empty() {
            return RemindMeDateTypes::Invalid;
        }

        // Three letter month 
        // TODO factor out regex strings to different date util file
        // TODO don't use group tags, might be faster?
        // TODO the captures unwrap shouldn't happen inside is_match but outside it will, should check anyway
        // TODO the regex should be unit tested
        let three_letter_month_reg = Regex::new(r"^(?<day>\d{1,2})(?<month>\w{3})(?<year>\d{2,4})$").unwrap();
        if three_letter_month_reg.is_match(str) {
            let three_letter_month_caps = three_letter_month_reg.captures(str).unwrap();

            let day = str::parse::<u32>(&get_cap_or_empty_string(&three_letter_month_caps, "day")).unwrap_or_default();
            let month = get_cap_or_empty_string(&three_letter_month_caps, "month");

            let month_num = month_to_number(&month);
            if month_num.is_none() { return RemindMeDateTypes::Invalid }

            let year = str::parse::<i32>(&get_cap_or_empty_string(&three_letter_month_caps, "year")).unwrap_or_default();


            return RemindMeDateTypes::ThreeLetterMonth{ d: day, mon: month_num.unwrap(), y: year};
        }

        // Specified time
        let specified_time_reg = Regex::new(r"^(?<hour>([0-2])([0-3]))(?<minute>([0-5])(\d))$").unwrap();
        if specified_time_reg.is_match(str) {
            let specified_time_caps = specified_time_reg.captures(str).unwrap();

            let hour = str::parse::<u32>(&get_cap_or_empty_string(&specified_time_caps, "hour")).unwrap_or_default();
            let minute = str::parse::<u32>(&get_cap_or_empty_string(&specified_time_caps, "minute")).unwrap_or_default();
            return RemindMeDateTypes::SpecifiedTime{ h: hour, min: minute};
        }

        // Is added time
        // TODO regex definitely needs unit testing, thanks god for the debugger and vim %
        let added_time_reg = Regex::new(r"^((?<year>\d+)[y])?((?<month>\d+)[M])?((?<day>\d+)[d])?((?<hour>\d+)[h])?((?<minute>\d+)[m])?$").unwrap();
        if added_time_reg.is_match(str) {
            let added_time_caps = added_time_reg.captures(str).unwrap();
            let year = str::parse::<i32>(&get_cap_or_empty_string(&added_time_caps, "year")).unwrap_or_default();
            let month = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "month")).unwrap_or_default();
            let day = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "day")).unwrap_or_default();
            let hour = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "hour")).unwrap_or_default();
            let minute = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "minute")).unwrap_or_default();

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

    // TODO check to see if we can fix the lifetime error and return string ref
    fn get_cap_or_empty_string(caps: &Captures<'_>, name: &str) -> String {
        return String::from(caps.name(name).map_or("", |m| m.as_str()));
    }
}