//! Handle P8SCII characters

pub fn char_to_utf8(p8char: u8) -> Option<&'static str> {
    match p8char {
        // The P8SCII character set as described by picotool.

        // Control codes
        0 => Some("\x00"),  // Terminate printing
        1 => Some("\x01"),  // Repeat next character P0 times
        2 => Some("\x02"),  // Draw solid background with color P0
        3 => Some("\x03"),  // Move cursor horizontally by P0-16 pixels
        4 => Some("\x04"),  // Move cursor vertically by P0-16 pixels
        5 => Some("\x05"),  // Move cursor by P0-16, P1-16 pixels
        6 => Some("\x06"),  // Special command
        7 => Some("\x07"),  // Audio command
        8 => Some("\x08"),  // Backspace
        9 => Some("\x09"),  // Tab
        10 => Some("\x0a"), // Newline
        11 => Some("\x0b"), // Decorate previous character command
        12 => Some("\x0c"), // Set foreground to color P0
        13 => Some("\x0d"), // Carriage return
        14 => Some("\x0e"), // Switch font defined at 0x5600
        15 => Some("\x0f"), // Switch font to default

        // Japanese punctuation
        16 => Some("▮"),  // Vertical rectangle
        17 => Some("■"),  // Filled square
        18 => Some("□"),  // Hollow square
        19 => Some("⁙"),  // Five dot
        20 => Some("⁘"),  // Four dot
        21 => Some("‖"),  // Pause
        22 => Some("◀"),  // Back
        23 => Some("▶"),  // Forward
        24 => Some("「"), // Japanese starting quote
        25 => Some("」"), // Japanese ending quote
        26 => Some("¥"),  // Yen sign
        27 => Some("•"),  // Interpunct
        28 => Some("、"), // Japanese comma
        29 => Some("。"), // Japanese full stop
        30 => Some("゛"), // Japanese dakuten
        31 => Some("゜"), // Japanese handakuten
        127 => Some("○"), // Hollow circle

        // Symbols
        128 => Some("█"),  // Rectangle
        129 => Some("▒"),  // Checkerboard
        130 => Some("🐱"), // Jelpi
        131 => Some("⬇️"), // Down key
        132 => Some("░"),  // Dot pattern
        133 => Some("✽"),  // Throwing star
        134 => Some("●"),  // Ball
        135 => Some("♥"),  // Heart
        136 => Some("☉"),  // Eye
        137 => Some("웃"), // Man
        138 => Some("⌂"),  // House
        139 => Some("⬅️"), // Left key
        140 => Some("😐"), // Face
        141 => Some("♪"),  // Musical note
        142 => Some("🅾️"), // O key
        143 => Some("◆"),  // Diamond
        144 => Some("…"),  // Ellipsis
        145 => Some("➡️"), // Right key
        146 => Some("★"),  // Five-pointed star
        147 => Some("⧗"),  // Hourglass
        148 => Some("⬆️"), // Up key
        149 => Some("ˇ"),  // Birds
        150 => Some("∧"),  // Sawtooth
        151 => Some("❎"), // X key
        152 => Some("▤"),  // Horiz lines
        153 => Some("▥"),  // Vert lines

        // Hiragana
        154 => Some("あ"), // Hiragana: a
        155 => Some("い"), // i
        156 => Some("う"), // u
        157 => Some("え"), // e
        158 => Some("お"), // o
        159 => Some("か"), // ka
        160 => Some("き"), // ki
        161 => Some("く"), // ku
        162 => Some("け"), // ke
        163 => Some("こ"), // ko
        164 => Some("さ"), // sa
        165 => Some("し"), // si
        166 => Some("す"), // su
        167 => Some("せ"), // se
        168 => Some("そ"), // so
        169 => Some("た"), // ta
        170 => Some("ち"), // chi
        171 => Some("つ"), // tsu
        172 => Some("て"), // te
        173 => Some("と"), // to
        174 => Some("な"), // na
        175 => Some("に"), // ni
        176 => Some("ぬ"), // nu
        177 => Some("ね"), // ne
        178 => Some("の"), // no
        179 => Some("は"), // ha
        180 => Some("ひ"), // hi
        181 => Some("ふ"), // phu
        182 => Some("へ"), // he
        183 => Some("ほ"), // ho
        184 => Some("ま"), // ma
        185 => Some("み"), // mi
        186 => Some("む"), // mu
        187 => Some("め"), // me
        188 => Some("も"), // mo
        189 => Some("や"), // ya
        190 => Some("ゆ"), // yu
        191 => Some("よ"), // yo
        192 => Some("ら"), // ra
        193 => Some("り"), // ri
        194 => Some("る"), // ru
        195 => Some("れ"), // re
        196 => Some("ろ"), // ro
        197 => Some("わ"), // wa
        198 => Some("を"), // wo
        199 => Some("ん"), // n
        200 => Some("っ"), // Hiragana sokuon
        201 => Some("ゃ"), // Hiragana digraphs: ya
        202 => Some("ゅ"), // yu
        203 => Some("ょ"), // yo

        // Katakana
        204 => Some("ア"), // Katakana: a
        205 => Some("イ"), // i
        206 => Some("ウ"), // u
        207 => Some("エ"), // e
        208 => Some("オ"), // o
        209 => Some("カ"), // ka
        210 => Some("キ"), // ki
        211 => Some("ク"), // ku
        212 => Some("ケ"), // ke
        213 => Some("コ"), // ko
        214 => Some("サ"), // sa
        215 => Some("シ"), // si
        216 => Some("ス"), // su
        217 => Some("セ"), // se
        218 => Some("ソ"), // so
        219 => Some("タ"), // ta
        220 => Some("チ"), // chi
        221 => Some("ツ"), // tsu
        222 => Some("テ"), // te
        223 => Some("ト"), // to
        224 => Some("ナ"), // na
        225 => Some("ニ"), // ni
        226 => Some("ヌ"), // nu
        227 => Some("ネ"), // ne
        228 => Some("ノ"), // no
        229 => Some("ハ"), // ha
        230 => Some("ヒ"), // hi
        231 => Some("フ"), // phu
        232 => Some("ヘ"), // he
        233 => Some("ホ"), // ho
        234 => Some("マ"), // ma
        235 => Some("ミ"), // mi
        236 => Some("ム"), // mu
        237 => Some("メ"), // me
        238 => Some("モ"), // mo
        239 => Some("ヤ"), // ya
        240 => Some("ユ"), // yu
        241 => Some("ヨ"), // yo
        242 => Some("ラ"), // ra
        243 => Some("リ"), // ri
        244 => Some("ル"), // ru
        245 => Some("レ"), // re
        246 => Some("ロ"), // ro
        247 => Some("ワ"), // wa
        248 => Some("ヲ"), // wo
        249 => Some("ン"), // n
        250 => Some("ッ"), // Katakana sokuon
        251 => Some("ャ"), // Katakana digraphs: ya
        252 => Some("ュ"), // yu
        253 => Some("ョ"), // yo

        // Remaining symbols
        254 => Some("◜"), // Left arc
        255 => Some("◝"), // Right arc
        _ => None,
    }
}

pub fn vec_to_utf8(p8chars: Vec<u8>) -> String {
    let mut accum = String::new();
    for p8char in p8chars {
        if let Some(utf8) = char_to_utf8(p8char) {
            accum.push_str(utf8);
        } else {
            accum.push(p8char as char);
        }
    }
    accum
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_x() {
        let mut v: Vec<u8> = Vec::new();
        v.extend(b"hit");
        v.push(145);
        v.extend(b" or ");
        v.push(151);
        v.extend(b" to go to next");
        let s = vec_to_utf8(v);
        assert_eq!("hit➡\u{fe0f} or ❎ to go to next", s);
    }
}
