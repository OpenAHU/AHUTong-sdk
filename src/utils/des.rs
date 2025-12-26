use std::convert::TryInto;

pub struct DES;

impl DES {
    /*
     * encrypt the string to string made up of hex return the encrypted string
     */
    pub fn str_enc(data: &str, first_key: &str, second_key: &str, third_key: &str) -> String {
        let data_u16: Vec<u16> = data.encode_utf16().collect();
        let length = data_u16.len();
        let mut enc_data = String::new();

        let first_key_bt = if !first_key.is_empty() { Some(Self::get_key_bytes(first_key)) } else { None };
        let second_key_bt = if !second_key.is_empty() { Some(Self::get_key_bytes(second_key)) } else { None };
        let third_key_bt = if !third_key.is_empty() { Some(Self::get_key_bytes(third_key)) } else { None };

        if length > 0 {
            if length < 4 {
                let bt = Self::str_to_bt(&data_u16);
                let enc_byte = Self::perform_encryption_core(bt, &first_key_bt, &second_key_bt, &third_key_bt);
                enc_data = Self::bt64_to_hex(&enc_byte);
            } else {
                let iterator = length / 4;
                let remainder = length % 4;

                for i in 0..iterator {
                    let temp_data = &data_u16[i * 4..(i * 4) + 4];
                    let temp_byte = Self::str_to_bt(temp_data);
                    let enc_byte = Self::perform_encryption_core(temp_byte, &first_key_bt, &second_key_bt, &third_key_bt);
                    enc_data.push_str(&Self::bt64_to_hex(&enc_byte));
                }

                if remainder > 0 {
                    let remainder_data = &data_u16[iterator * 4..length];
                    let temp_byte = Self::str_to_bt(remainder_data);
                    let enc_byte = Self::perform_encryption_core(temp_byte, &first_key_bt, &second_key_bt, &third_key_bt);
                    enc_data.push_str(&Self::bt64_to_hex(&enc_byte));
                }
            }
        }
        enc_data
    }

    /*
     * decrypt the encrypted string to the original string
     *
     * return the original string
     */

    pub fn str_dec(data: &str, first_key: &str, second_key: &str, third_key: &str) -> String {
        let length = data.len();
        let mut all_u16s = Vec::new();

        let first_key_bt = if !first_key.is_empty() { Some(Self::get_key_bytes(first_key)) } else { None };
        let second_key_bt = if !second_key.is_empty() { Some(Self::get_key_bytes(second_key)) } else { None };
        let third_key_bt = if !third_key.is_empty() { Some(Self::get_key_bytes(third_key)) } else { None };

        let iterator = length / 16;
        for i in 0..iterator {
            let temp_data = &data[i * 16..(i * 16) + 16];
            let str_byte = Self::hex_to_bt64(temp_data);

            let mut int_byte = vec![0; 64];
            for j in 0..64 {
                let c = str_byte.chars().nth(j).unwrap();
                int_byte[j] = c.to_digit(10).unwrap() as i32;
            }

            let dec_byte = Self::perform_decryption_core(int_byte, &first_key_bt, &second_key_bt, &third_key_bt);
            let u16s = Self::byte_to_u16s(&dec_byte);
            all_u16s.extend(u16s);
        }
        
        String::from_utf16(&all_u16s).unwrap_or_default()
    }


    fn perform_encryption_core(
        mut temp_bt: Vec<i32>,
        first: &Option<Vec<Vec<i32>>>,
        second: &Option<Vec<Vec<i32>>>,
        third: &Option<Vec<Vec<i32>>>
    ) -> Vec<i32> {
        if let (Some(k1), Some(k2), Some(k3)) = (first, second, third) {
            for x in 0..k1.len() { temp_bt = Self::enc(temp_bt, &k1[x]); }
            for y in 0..k2.len() { temp_bt = Self::enc(temp_bt, &k2[y]); }
            for z in 0..k3.len() { temp_bt = Self::enc(temp_bt, &k3[z]); }
        } else if let (Some(k1), Some(k2)) = (first, second) {
            for x in 0..k1.len() { temp_bt = Self::enc(temp_bt, &k1[x]); }
            for y in 0..k2.len() { temp_bt = Self::enc(temp_bt, &k2[y]); }
        } else if let Some(k1) = first {
            for x in 0..k1.len() { temp_bt = Self::enc(temp_bt, &k1[x]); }
        }
        temp_bt
    }

    fn perform_decryption_core(
        mut temp_bt: Vec<i32>,
        first: &Option<Vec<Vec<i32>>>,
        second: &Option<Vec<Vec<i32>>>,
        third: &Option<Vec<Vec<i32>>>
    ) -> Vec<i32> {
        if let (Some(k1), Some(k2), Some(k3)) = (first, second, third) {
            for x in (0..k3.len()).rev() { temp_bt = Self::dec(temp_bt, &k3[x]); }
            for y in (0..k2.len()).rev() { temp_bt = Self::dec(temp_bt, &k2[y]); }
            for z in (0..k1.len()).rev() { temp_bt = Self::dec(temp_bt, &k1[z]); }
        } else if let (Some(k1), Some(k2)) = (first, second) {
            for x in (0..k2.len()).rev() { temp_bt = Self::dec(temp_bt, &k2[x]); }
            for y in (0..k1.len()).rev() { temp_bt = Self::dec(temp_bt, &k1[y]); }
        } else if let Some(k1) = first {
            for x in (0..k1.len()).rev() { temp_bt = Self::dec(temp_bt, &k1[x]); }
        }
        temp_bt
    }

    /*
     * chang the string into the bit array
     *
     * return bit array(it's length % 64 = 0)
     */
    fn get_key_bytes(key: &str) -> Vec<Vec<i32>> {
        let key_u16: Vec<u16> = key.encode_utf16().collect();
        let length = key_u16.len();
        let iterator = length / 4;
        let remainder = length % 4;
        let mut key_bytes = Vec::new();

        for i in 0..iterator {
            key_bytes.push(Self::str_to_bt(&key_u16[i * 4..(i * 4) + 4]));
        }
        if remainder > 0 {
            key_bytes.push(Self::str_to_bt(&key_u16[iterator * 4..length]));
        }
        key_bytes
    }

    /*
     * chang the string(it's length <= 4) into the bit array
     *
     * return bit array(it's length = 64)
     */
    fn str_to_bt(chars: &[u16]) -> Vec<i32> {
        let length = chars.len();
        let mut bt = vec![0; 64];

        if length < 4 {
            for (i, &c) in chars.iter().enumerate() {
                let k = c as i32;
                for j in 0..16 {
                    let mut pow = 1;
                    for _ in (j + 1)..16 { pow *= 2; }
                    bt[16 * i + j] = (k / pow) % 2;
                }
            }
            for p in length..4 {
                let k = 0;
                for q in 0..16 {
                    let mut pow = 1;
                    for _ in (q + 1)..16 { pow *= 2; }
                    bt[16 * p + q] = (k / pow) % 2;
                }
            }
        } else {
            for i in 0..4 {
                let k = chars[i] as i32;
                for j in 0..16 {
                    let mut pow = 1;
                    for _ in (j + 1)..16 { pow *= 2; }
                    bt[16 * i + j] = (k / pow) % 2;
                }
            }
        }
        bt
    }

    /*
     * chang the bit(it's length = 64) into the string
     *
     * return string
     */
    fn byte_to_u16s(byte_data: &[i32]) -> Vec<u16> {
        let mut res = Vec::new();
        for i in 0..4 {
            let mut count = 0;
            for j in 0..16 {
                let mut pow = 1;
                for _ in (j + 1)..16 { pow *= 2; }
                count += byte_data[16 * i + j] * pow;
            }
            if count != 0 {
                res.push(count as u16);
            }
        }
        res
    }
    
    // For compatibility if needed, but not used in new dec logic
    #[allow(dead_code)]
    fn byte_to_string(byte_data: &[i32]) -> String {
        let u16s = Self::byte_to_u16s(byte_data);
        String::from_utf16(&u16s).unwrap_or_default()
    }

    /*
     * chang the bit(it's length = 4) into the hex
     *
     * return hex
     */
    fn bt64_to_hex(byte_data: &[i32]) -> String {
        let mut hex = String::new();
        for i in 0..16 {
            let mut bt = String::new();
            for j in 0..4 {
                bt.push_str(&byte_data[i * 4 + j].to_string());
            }
            hex.push_str(&Self::bt4_to_hex(&bt));
        }
        hex
    }

    /*
     * chang the hex into the bit(it's length = 4)
     *
     * return the bit(it's length = 4)
     */

    fn hex_to_bt64(hex: &str) -> String {
        let mut binary = String::new();
        for i in 0..16 {
            binary.push_str(&Self::hex_to_bt4(&hex[i..i+1]));
        }
        binary
    }

    fn bt4_to_hex(binary: &str) -> String {
        match binary {
            "0000" => "0", "0001" => "1", "0010" => "2", "0011" => "3",
            "0100" => "4", "0101" => "5", "0110" => "6", "0111" => "7",
            "1000" => "8", "1001" => "9", "1010" => "A", "1011" => "B",
            "1100" => "C", "1101" => "D", "1110" => "E", "1111" => "F",
            _ => "",
        }.to_string()
    }

    fn hex_to_bt4(hex: &str) -> String {
        match hex.to_uppercase().as_str() {
            "0" => "0000", "1" => "0001", "2" => "0010", "3" => "0011",
            "4" => "0100", "5" => "0101", "6" => "0110", "7" => "0111",
            "8" => "1000", "9" => "1001", "A" => "1010", "B" => "1011",
            "C" => "1100", "D" => "1101", "E" => "1110", "F" => "1111",
            _ => "",
        }.to_string()
    }

    /*
     * the 64 bit des core arithmetic
     */
    fn enc(data_byte: Vec<i32>, key_byte: &[i32]) -> Vec<i32> {
        let keys = Self::generate_keys(key_byte);
        let ip_byte = Self::init_permute(&data_byte);
        let mut ip_left = ip_byte[0..32].to_vec();
        let mut ip_right = ip_byte[32..64].to_vec();
        let mut temp_left = vec![0; 32];

        for i in 0..16 {
            temp_left.copy_from_slice(&ip_left);
            ip_left.copy_from_slice(&ip_right);

            let key = &keys[i];
            let expanded = Self::expand_permute(&ip_right);
            let xor1 = Self::xor(&expanded, key);
            let sbox = Self::s_box_permute(&xor1);
            let p_perm = Self::p_permute(&sbox);
            let xor2 = Self::xor(&p_perm, &temp_left);

            ip_right.copy_from_slice(&xor2);
        }

        let mut final_data = vec![0; 64];
        for i in 0..32 {
            final_data[i] = ip_right[i];
            final_data[32 + i] = ip_left[i];
        }
        Self::finally_permute(&final_data)
    }

    fn dec(data_byte: Vec<i32>, key_byte: &[i32]) -> Vec<i32> {
        let keys = Self::generate_keys(key_byte);
        let ip_byte = Self::init_permute(&data_byte);
        let mut ip_left = ip_byte[0..32].to_vec();
        let mut ip_right = ip_byte[32..64].to_vec();
        let mut temp_left = vec![0; 32];

        for i in (0..16).rev() {
            temp_left.copy_from_slice(&ip_left);
            ip_left.copy_from_slice(&ip_right);

            let key = &keys[i];
            let expanded = Self::expand_permute(&ip_right);
            let xor1 = Self::xor(&expanded, key);
            let sbox = Self::s_box_permute(&xor1);
            let p_perm = Self::p_permute(&sbox);
            let xor2 = Self::xor(&p_perm, &temp_left);

            ip_right.copy_from_slice(&xor2);
        }

        let mut final_data = vec![0; 64];
        for i in 0..32 {
            final_data[i] = ip_right[i];
            final_data[32 + i] = ip_left[i];
        }
        Self::finally_permute(&final_data)
    }


    fn init_permute(original_data: &[i32]) -> Vec<i32> {
        let mut ip_byte = vec![0; 64];
        let (mut m, mut n) = (1, 0);
        for i in 0..4 {
            let mut k = 0;
            for j in (0..=7).rev() {
                ip_byte[i * 8 + k] = original_data[j * 8 + m];
                ip_byte[i * 8 + k + 32] = original_data[j * 8 + n];
                k += 1;
            }
            m += 2;
            n += 2;
        }
        ip_byte
    }

    fn expand_permute(right_data: &[i32]) -> Vec<i32> {
        let mut ep_byte = vec![0; 48];
        for i in 0..8 {
            if i == 0 {
                ep_byte[i * 6 + 0] = right_data[31];
            } else {
                ep_byte[i * 6 + 0] = right_data[i * 4 - 1];
            }
            ep_byte[i * 6 + 1] = right_data[i * 4 + 0];
            ep_byte[i * 6 + 2] = right_data[i * 4 + 1];
            ep_byte[i * 6 + 3] = right_data[i * 4 + 2];
            ep_byte[i * 6 + 4] = right_data[i * 4 + 3];
            if i == 7 {
                ep_byte[i * 6 + 5] = right_data[0];
            } else {
                ep_byte[i * 6 + 5] = right_data[i * 4 + 4];
            }
        }
        ep_byte
    }

    fn xor(byte_one: &[i32], byte_two: &[i32]) -> Vec<i32> {
        byte_one.iter().zip(byte_two.iter()).map(|(a, b)| a ^ b).collect()
    }

    fn s_box_permute(expand_byte: &[i32]) -> Vec<i32> {
        let mut s_box_byte = vec![0; 32];

        let s1 = [
            [14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7],
            [0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8],
            [4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0],
            [15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13]
        ];
        let s2 = [
            [15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10],
            [3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5],
            [0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15],
            [13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9]
        ];
        let s3 = [
            [10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8],
            [13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1],
            [13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7],
            [1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12]
        ];
        let s4 = [
            [7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15],
            [13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9],
            [10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4],
            [3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14]
        ];
        let s5 = [
            [2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9],
            [14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6],
            [4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14],
            [11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3]
        ];
        let s6 = [
            [12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11],
            [10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8],
            [9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6],
            [4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13]
        ];
        let s7 = [
            [4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1],
            [13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6],
            [1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2],
            [6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12]
        ];
        let s8 = [
            [13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7],
            [1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2],
            [7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8],
            [2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11]
        ];

        for m in 0..8 {
            let i = expand_byte[m * 6 + 0] * 2 + expand_byte[m * 6 + 5];
            let j = expand_byte[m * 6 + 1] * 8
                + expand_byte[m * 6 + 2] * 4
                + expand_byte[m * 6 + 3] * 2
                + expand_byte[m * 6 + 4];

            let val = match m {
                0 => s1[i as usize][j as usize],
                1 => s2[i as usize][j as usize],
                2 => s3[i as usize][j as usize],
                3 => s4[i as usize][j as usize],
                4 => s5[i as usize][j as usize],
                5 => s6[i as usize][j as usize],
                6 => s7[i as usize][j as usize],
                7 => s8[i as usize][j as usize],
                _ => 0,
            };

            let binary = Self::get_box_binary(val);
            s_box_byte[m * 4 + 0] = binary.chars().nth(0).unwrap().to_digit(10).unwrap() as i32;
            s_box_byte[m * 4 + 1] = binary.chars().nth(1).unwrap().to_digit(10).unwrap() as i32;
            s_box_byte[m * 4 + 2] = binary.chars().nth(2).unwrap().to_digit(10).unwrap() as i32;
            s_box_byte[m * 4 + 3] = binary.chars().nth(3).unwrap().to_digit(10).unwrap() as i32;
        }
        s_box_byte
    }

    fn get_box_binary(i: i32) -> String {
        match i {
            0 => "0000", 1 => "0001", 2 => "0010", 3 => "0011",
            4 => "0100", 5 => "0101", 6 => "0110", 7 => "0111",
            8 => "1000", 9 => "1001", 10 => "1010", 11 => "1011",
            12 => "1100", 13 => "1101", 14 => "1110", 15 => "1111",
            _ => "0000"
        }.to_string()
    }

    fn p_permute(s_box_byte: &[i32]) -> Vec<i32> {
        let mut p = vec![0; 32];
        let map = [
            15, 6, 19, 20, 28, 11, 27, 16,
            0, 14, 22, 25, 4, 17, 30, 9,
            1, 7, 23, 13, 31, 26, 2, 8,
            18, 12, 29, 5, 21, 10, 3, 24
        ];
        for i in 0..32 {
            p[i] = s_box_byte[map[i]];
        }
        p
    }

    fn finally_permute(end_byte: &[i32]) -> Vec<i32> {
        let mut fp = vec![0; 64];
        let map = [
            39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22, 62, 30,
            37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28,
            35, 3, 43, 11, 51, 19, 59, 27, 34, 2, 42, 10, 50, 18, 58, 26,
            33, 1, 41, 9, 49, 17, 57, 25, 32, 0, 40, 8, 48, 16, 56, 24
        ];
        for i in 0..64 {
            fp[i] = end_byte[map[i]];
        }
        fp
    }

    /*
     * generate 16 keys for xor
     */

    fn generate_keys(key_byte: &[i32]) -> Vec<Vec<i32>> {
        let mut key = vec![0; 56];
        let mut keys = vec![vec![0; 48]; 16];
        let loop_shifts = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];

        for i in 0..7 {
            let mut k = 7;
            for j in 0..8 {
                key[i * 8 + j] = key_byte[8 * k + i];
                if k > 0 { k -= 1; }
            }
        }

        for i in 0..16 {
            for _ in 0..loop_shifts[i] {
                let temp_left = key[0];
                let temp_right = key[28];
                for k in 0..27 {
                    key[k] = key[k + 1];
                    key[28 + k] = key[29 + k];
                }
                key[27] = temp_left;
                key[55] = temp_right;
            }

            let mut temp_key = vec![0; 48];
            temp_key[0] = key[13]; temp_key[1] = key[16]; temp_key[2] = key[10]; temp_key[3] = key[23];
            temp_key[4] = key[0];  temp_key[5] = key[4];  temp_key[6] = key[2];  temp_key[7] = key[27];
            temp_key[8] = key[14]; temp_key[9] = key[5];  temp_key[10] = key[20]; temp_key[11] = key[9];
            temp_key[12] = key[22]; temp_key[13] = key[18]; temp_key[14] = key[11]; temp_key[15] = key[3];
            temp_key[16] = key[25]; temp_key[17] = key[7];  temp_key[18] = key[15]; temp_key[19] = key[6];
            temp_key[20] = key[26]; temp_key[21] = key[19]; temp_key[22] = key[12]; temp_key[23] = key[1];
            temp_key[24] = key[40]; temp_key[25] = key[51]; temp_key[26] = key[30]; temp_key[27] = key[36];
            temp_key[28] = key[46]; temp_key[29] = key[54]; temp_key[30] = key[29]; temp_key[31] = key[39];
            temp_key[32] = key[50]; temp_key[33] = key[44]; temp_key[34] = key[32]; temp_key[35] = key[47];
            temp_key[36] = key[43]; temp_key[37] = key[48]; temp_key[38] = key[38]; temp_key[39] = key[55];
            temp_key[40] = key[33]; temp_key[41] = key[52]; temp_key[42] = key[45]; temp_key[43] = key[41];
            temp_key[44] = key[49]; temp_key[45] = key[35]; temp_key[46] = key[28]; temp_key[47] = key[31];

            keys[i] = temp_key;
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        // "123" with keys
        let res = DES::str_enc("123", "1", "2", "3");
        println!("Encrypted 123: {}", res);
    }

    #[test]
    fn test_ascii_4() {
        // "1234" with keys
        let res = DES::str_enc("1234", "1", "2", "3");
        println!("Encrypted 1234: {}", res);
    }

    #[test]
    fn test_unicode_crash() {
        // "测试" (6 bytes in UTF-8, 2 chars)
        // This should NOT crash anymore
        let res = DES::str_enc("测试", "1", "2", "3");
        println!("Encrypted Unicode: {}", res);
    }
    
    #[test]
    fn test_decrypt_unicode() {
        let original = "测试123";
        let encrypted = DES::str_enc(original, "1", "2", "3");
        let decrypted = DES::str_dec(&encrypted, "1", "2", "3");
        assert_eq!(original, decrypted);
    }
}
