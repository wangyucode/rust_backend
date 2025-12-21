use uuid::Uuid;

/// Generate a short UUID (base62 encoded)
pub fn generate_short_uuid() -> String {
    // Generate a standard UUID and remove hyphens
    let uuid_str = Uuid::new_v4().to_string().replace("-", "");

    let mut short_id = String::with_capacity(4);

    // Process the UUID in 4 groups of 8 characters
    for i in 0..4 {
        // Get 8-character substring
        let start = i * 8;
        let end = start + 8;
        let str_chunk = &uuid_str[start..end];

        // Convert hex string to u32
        let x = u32::from_str_radix(str_chunk, 16).unwrap();

        // Modulo 36 and convert to base36 (lowercase)
        short_id.push_str(&radix36_encode(x % 36));
    }

    short_id
}

// Helper function to encode a number to base36 (lowercase)
fn radix36_encode(mut num: u32) -> String {
    if num == 0 {
        return "0".to_string();
    }

    const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = String::new();

    while num > 0 {
        let remainder = (num % 36) as usize;
        result.insert(0, CHARSET[remainder] as char);
        num /= 36;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_short_uuid_length() {
        // Test that the generated UUID is exactly 4 characters long
        let uuid = generate_short_uuid();
        assert_eq!(uuid.len(), 4);
    }

    #[test]
    fn test_generate_short_uuid_chars() {
        // Test that the generated UUID only contains valid base36 characters (lowercase)
        let uuid = generate_short_uuid();
        assert!(
            uuid.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn test_generate_short_uuid_lowercase() {
        // Test that the generated UUID is all lowercase
        let uuid = generate_short_uuid();
        assert_eq!(uuid, uuid.to_lowercase());
    }

    #[test]
    fn test_generate_short_uuid_uniqueness() {
        // Test that multiple calls generate different UUIDs
        let uuid1 = generate_short_uuid();
        let uuid2 = generate_short_uuid();
        let uuid3 = generate_short_uuid();

        // Note: With only 4 characters (36^4 = 1,679,616 possibilities), collisions are possible
        // but unlikely in this small test
        assert_ne!(uuid1, uuid2);
        assert_ne!(uuid1, uuid3);
        assert_ne!(uuid2, uuid3);
    }

    #[test]
    fn test_radix36_encode() {
        // Test the base36 encoding helper function
        assert_eq!(radix36_encode(0), "0");
        assert_eq!(radix36_encode(9), "9");
        assert_eq!(radix36_encode(10), "a");
        assert_eq!(radix36_encode(35), "z");
        assert_eq!(radix36_encode(36), "10");
        assert_eq!(radix36_encode(37), "11");
        assert_eq!(radix36_encode(61), "1p");
    }
}
