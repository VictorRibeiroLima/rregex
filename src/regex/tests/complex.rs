use crate::regex::tests::find::find;

#[test]
fn brazilian_cellphone_regex() {
    let re = r"^\d{2}\d{2}9\d{8}$";
    assert_eq!(find(re, "5511987654321"), Some(13));
    assert_eq!(find(re, "5521987654321"), Some(13));
    assert_eq!(find(re, "559987654321"), None); // missing initial digits
    assert_eq!(find(re, "551111987654321"), None); // extra leading digit
}

#[test]
fn email_regex() {
    let re = r"^[\w.+-]+@([\w-]+\.)+[\w-]{2,}$";
    assert_eq!(find(re, "user@example.com"), Some(16));
    assert_eq!(find(re, "user@sub.example.com"), Some(20));
    assert_eq!(find(re, "user@com"), None); // domain too short
    assert_eq!(find(re, "@example.com"), None); // missing local part
}

#[test]
fn uuid_v4_regex() {
    let re =
        r"^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[4][0-9A-Fa-f]{3}-[89AB][0-9A-Fa-f]{3}-[0-9A-Fa-f]{12}$";
    assert_eq!(find(re, "8a4d025e-86ac-4273-8bd0-7fd367fdfa8e"), Some(36));
}
