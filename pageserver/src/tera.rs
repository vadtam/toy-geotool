use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rocket_dyn_templates::{
    tera::Value as TeraValue,
    tera::Result as TeraResult,
};

use crate::point::*;

pub fn tera_points_f32_to_json_string(points: &Vec<PointF32>) -> String {
    let mut ss: String = "[".to_string();
    let n_points = points.len();
    if n_points == 0 {
        return "[]".to_string();
    }
    let mut idx: usize = 1;
    for point in points.iter() {
        let time: i64 = (point.time as i64) * 1000;
        let val_ss: String = format!("x:{},y:{}", time, point.value);
        ss += ("{".to_owned() + &val_ss + "}").as_str();
        if idx != n_points {
            ss += ",";
            idx += 1;
        } else {
            ss += "]";
        }
    }
    ss
}

pub fn tera_points_f64f64_to_json_string(points: &Vec<PointF64F64>) -> String {
    let mut ss: String = "[".to_string();
    let n_points = points.len();
    if n_points == 0 {
        return "[]".to_string();
    }
    let mut idx: usize = 1;
    for point in points.iter() {
        let val_ss: String = format!("x:{},y:{}", point.x, point.y);
        ss += ("{".to_owned() + &val_ss + "}").as_str();
        if idx != n_points {
            ss += ",";
            idx += 1;
        } else {
            ss += "]";
        }
    }
    ss
}

pub fn tera_points_f64f32_to_json_string(points: &Vec<PointF64F32>) -> String {
    let mut ss: String = "[".to_string();
    let n_points = points.len();
    if n_points == 0 {
        return "[]".to_string();
    }
    let mut idx: usize = 1;
    for point in points.iter() {
        let val_ss: String = format!("x:{},y:{}", point.x, point.y);
        ss += ("{".to_owned() + &val_ss + "}").as_str();
        if idx != n_points {
            ss += ",";
            idx += 1;
        } else {
            ss += "]";
        }
    }
    ss
}

pub fn tera_points_f32f32_to_json_string(points: &Vec<PointF32F32>) -> String {
    let mut ss: String = "[".to_string();
    let n_points = points.len();
    if n_points == 0 {
        return "[]".to_string();
    }
    let mut idx: usize = 1;
    for point in points.iter() {
        let val_ss: String = format!("x:{},y:{}", point.x, point.y);
        ss += ("{".to_owned() + &val_ss + "}").as_str();
        if idx != n_points {
            ss += ",";
            idx += 1;
        } else {
            ss += "]";
        }
    }
    ss
}

pub fn tera_last_active(value: &TeraValue, _args: &HashMap<String, TeraValue>) ->
        TeraResult<TeraValue> {
    let duration_res = SystemTime::now().duration_since(UNIX_EPOCH);
    if duration_res.is_ok() {
        let now: u64 = duration_res.unwrap().as_secs();
        let last_active: u64 = value.as_u64().unwrap();
        let elapsed_time = now - last_active;
        let day = 60*60*24;
        if last_active == 0 {
            Ok(TeraValue::String("Never".to_string()))
        } else if elapsed_time < 3 * day {
            Ok(TeraValue::String("Recently".to_string()))
        } else if elapsed_time < 7 * day {
            Ok(TeraValue::String("Within a week".to_string()))
        } else if elapsed_time < 30 * day {
            Ok(TeraValue::String("Within a month".to_string()))
        } else {
            Ok(TeraValue::String("F64 time ago".to_string()))
        }
    } else {
        Ok(TeraValue::String("Server Error".to_string()))
    }
}

pub fn tera_user_category_activator(args: &HashMap<String, TeraValue>) -> TeraResult<TeraValue> {
    let companyid: &str = args.get("companyid").unwrap().as_str().unwrap();
    let is_new_mode: bool = args.get("is_new_mode").unwrap().as_bool().unwrap();
    let xusercategory: &str = args.get("xusercategory").unwrap().as_str().unwrap();
    let refcategory: &str = args.get("refcategory").unwrap().as_str().unwrap();
    // decide on "checked"
    if companyid == "geomec" {
        if is_new_mode {
            Ok(TeraValue::String("".to_string()))
        } else {
            if xusercategory == refcategory {
                Ok(TeraValue::String(" checked".to_string()))
            } else {
                Ok(TeraValue::String("".to_string()))
            }
        }
    } else {
        if is_new_mode {
            if xusercategory == "User" {
                Ok(TeraValue::String(" checked ".to_string()))
            } else {
                Ok(TeraValue::String("".to_string()))
            }
        } else {
            if xusercategory == refcategory {
                Ok(TeraValue::String(" checked".to_string()))
            } else {
                Ok(TeraValue::String("".to_string()))
            }
        }
    }
}

pub fn tera_can_delete_user(args: &HashMap<String, TeraValue>) -> TeraResult<TeraValue> {
    let xusercategory: &str = args.get("xusercategory").unwrap().as_str().unwrap();
    let xuserid: &str = args.get("xuserid").unwrap().as_str().unwrap();
    let userid: &str = args.get("userid").unwrap().as_str().unwrap();
    let companyid: &str = args.get("companyid").unwrap().as_str().unwrap();
    if companyid == "geomec" {
        if(xusercategory == "Admin") && (xuserid != userid) {
            Ok(TeraValue::Bool(false))
        } else {
            Ok(TeraValue::Bool(true))
        }
    } else {
        Ok(TeraValue::Bool(true))
    }
}

