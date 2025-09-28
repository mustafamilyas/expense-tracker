/*
Format
10000
10.000
Rp 10.000
Rp10.000
Rp 10,000
Rp10,000
Rp. 1.234.567
Rp.1.234.567
Rp 5000
*/
use anyhow::Result;

pub fn parse_price(input: &str) -> Result<f64> {
    let input = input.trim();
    let input = input.replace('.', "").replace(',', "");
    // Remove "Rp" prefix if exists
    let input = if input.to_lowercase().starts_with("rp") {
        input[2..].trim().to_string()
    } else {
        input
    };
    // Remove dots and commas
    // Parse to f64
    let price: f64 = input
        .parse()
        .map_err(|_| anyhow::anyhow!("Failed to parse price: {}", input))?;
    if price < 0.0 {
        return Err(anyhow::anyhow!("Price cannot be negative: {}", input));
    }
    Ok(price)
}

// Format price to string with dot as thousand separator
// 10000 -> 10.000
pub fn format_price(price: f64) -> String {
    let mut price_str = format!("{:.0}", price);
    let mut result = String::new();
    while price_str.len() > 3 {
        let len = price_str.len();
        let chunk = &price_str[len - 3..];
        result = format!(".{}{}", chunk, result);
        price_str = price_str[..len - 3].to_string();
    }
    if !price_str.is_empty() {
        result = format!("{}{}", price_str, result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_price() {
        let cases = vec![
            ("10000", 10000.0),
            ("10.000", 10000.0),
            ("Rp 10.000", 10000.0),
            ("Rp10.000", 10000.0),
            ("Rp 10,000", 10000.0),
            ("Rp10,000", 10000.0),
            ("  Rp  1.234.567  ", 1234567.0),
            ("0", 0.0),
            ("  5000  ", 5000.0),
            ("Rp. 5000", 5000.0),
            ("Rp.1.234.567", 1234567.0),
            ("Rp. 1,234,567", 1234567.0),
            ("Rp 1,234,567", 1234567.0),
        ];
        for (input, expected) in cases {
            let result = parse_price(input).unwrap();
            assert_eq!(result, expected, "Failed on input: {}", input);
        }
    }
    #[test]
    fn test_parse_price_invalid() {
        let cases = vec!["-10000", "abc", "Rp -5000"];
        for input in cases {
            let result = parse_price(input);
            assert!(result.is_err(), "Expected error on input: {}", input);
        }
    }

    #[test]
    fn test_format_price() {
        let cases = vec![
            (10000.0, "10.000"),
            (1234567.0, "1.234.567"),
            (0.0, "0"),
            (5000.0, "5.000"),
            (100.0, "100"),
        ];
        for (input, expected) in cases {
            let result = format_price(input);
            assert_eq!(result, expected, "Failed on input: {}", input);
        }
    }
}
