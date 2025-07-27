use std::num::NonZeroUsize;

use num_format::{Locale, ToFormattedString as _};

/// 浮動小数点数 `x` を、整数部を3桁区切りしつつ小数点以下を `decimals` 桁に丸めて文字列化します。
/// 負の0 (`-0.0`) を含む負数でも符号を正しく付加し、大きな整数部も `i64` の範囲で処理します。
pub(crate) fn format_float_with_commas(x: f64, decimals: NonZeroUsize) -> String {
    // 桁数（>= 1）
    let decimals = decimals.get();

    // 符号を保持
    let is_negative = x.is_sign_negative();

    // 絶対値を指定桁数で文字列化
    // ここで decimals は必ず 1 以上
    let abs_str = format!("{:.*}", decimals, x.abs());

    // 小数点で分割（decimals >= 1 なので必ず小数点は存在する）
    let (int_part, frac_part) = abs_str.split_once('.').unwrap();

    // 整数部を i64 にパースしてカンマ区切り
    // （非常に大きい場合は BigInt などを検討）
    let int_formatted = int_part
        .parse::<i64>()
        .unwrap()
        .to_formatted_string(&Locale::en);

    // 整数部と小数部を再連結
    let result = format!("{int_formatted}.{frac_part}");

    // 負数なら符号を付けて返す
    if is_negative {
        format!("-{result}")
    } else {
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_format_float_with_commas_basic() {
        let decimals1 = NonZeroUsize::new(1).unwrap();
        let decimals3 = NonZeroUsize::new(3).unwrap();

        // 正の数, 小数点以下1桁
        assert_eq!(format_float_with_commas(12345.6789, decimals1), "12,345.7");
        // 負の数, 小数点以下1桁
        assert_eq!(format_float_with_commas(-0.1, decimals1), "-0.1");

        // 正の数, 小数点以下3桁 (繰り上がりが発生)
        // 12,345.6789 → 12,345.679
        assert_eq!(
            format_float_with_commas(12345.6789, decimals3),
            "12,345.679"
        );

        // 負の数, 小数点以下3桁, 非常に小さい値
        // -0.0004 → -0.000 (丸め)
        assert_eq!(format_float_with_commas(-0.0004, decimals3), "-0.000");

        // 負の0 (is_sign_negative が true となる -0.0)
        // 小数点以下3桁 → -0.000
        assert_eq!(format_float_with_commas(-0.0, decimals3), "-0.000");
    }

    #[test]
    fn test_format_float_with_commas_large() {
        let decimals3 = NonZeroUsize::new(3).unwrap();

        // 非常に大きい数で繰り上がりあり (999,999,999.9999 → 1,000,000,000.000)
        assert_eq!(
            format_float_with_commas(999999999.9999, decimals3),
            "1,000,000,000.000"
        );

        // 10桁以上の数
        assert_eq!(
            format_float_with_commas(1234567890123.001, decimals3),
            "1,234,567,890,123.001"
        );
    }
}
