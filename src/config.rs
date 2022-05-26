use std::str::FromStr;

pub struct Config {
    pub worker_amt: usize,
    pub input_path: String
}

pub fn get_config(args: &Vec<String>) -> Config {
    let worker_amt: usize = get_args(&args, 1, 1);
    let input_path = get_args(&args, 2, String::from("input.csv"));
    Config { worker_amt, input_path }
}

fn get_args<T>(args: &Vec<String>, idx: usize, default: T) -> T 
where T: FromStr
{
    if args.len() > idx {
        let s_arg = (*(&args[idx])).clone();
        return s_arg.parse::<T>().unwrap_or(default);
    }
    default
}