//! Every preset version Pairee has shipped, by hash (line endings ignored).
//!
//! A `keymaps/<name>.toml` identical to one of them was written by Pairee
//! and never edited, so the seeder may replace it with the current version.
//! Add the hash of each new shipped version here (the test below lists the
//! missing ones).

/// blake3 hashes of shipped presets, with the preset each belongs to.
const SHIPPED: &[&str] = &[
    "0469af70bbaac69553433437c194ba9994969b24b3c31c5124cb13dd39915317", // neovim
    "0dcdde5cc982629af86eb1d7eda4db9fe39fd0915186bb2c43ee2c4d2ebc3948", // neovim
    "11ef8aa9fb1c8a00c73ba69d0b2ab3a50fc486a5a610600b5d269499e2e74c8a", // neovim
    "63bfee0ce88840d9451a463fc4039f2d19da952a98eb01121df070f4d9cf9ae9", // neovim
    "763d6cbb5f33df3817593536e42fd9fd72ad628fbb55fe6f9decdb22bd7e7610", // neovim
    "8f853ed8be605576234374bdb4a7989b2683eec048864546ba18045269697a8f", // neovim
    "93ca8430d5bb2a5cfb92e500da94d32451255a699ff4217b110e8b618975f3e6", // neovim
    "9da0314a87960b383beca536578fc31b9e4eed86a86433e25d283be55a70edf1", // neovim
    "a7efccc6eba6f7e20c1a82b9b635990181f57c8b2f5176e1c73a9fefb7c6a208", // neovim
    "ba9fe5c7765ea1104ec9975a702011e8e7d159a6d2e9998462aa6e68b4ae289d", // neovim
    "ca8da965a735df475cb9cbe428ee3bbf4963dfeb7e10efb590b17b17fd2caecb", // neovim
    "cf4878be0186470acb4a4f65bda8c667aa82468d1602c3570de60209da6bb040", // neovim
    "f0526ff86e81c72ccfc8bddbc9508488382cde0ca7b27c85c2c6b9b3837bf57e", // neovim
    "f129de16f9f309f4fd694432e9bda5fec407a5f30caa86e4327cb2a6c3e03ba6", // neovim
    "f38be36894b0834dadce3726bc4224a3f302390c3897ab77cbd77fca9e5fac50", // neovim
    "f4769f1d97860f9d4f25baee07dce576331a6c118ff984037e9b0f10c5bd0ada", // neovim
    "0b3dde03ee989e123edbfebb77e984e7f4f5d3be96b788da1e54aeda102c6971", // norton
    "0b87e7f666565c85a416513bae3c05eb665c51e89d2c3713ee6d57e8b757f5a1", // norton
    "1c2c2b2da5d2a8d0b9073609d211cd671f232cb9daa569a6ea03b14a76995066", // norton
    "33450c2a55c8c3e82179816d586404ca872e20e0e136228ef6a618dcd847994b", // norton
    "34c081a731ce58d12b5aff115bbce43960599035d1c8aea29b50de07cc9ae530", // norton
    "528330ce2c1e3c22c8d1b501247f5a9409f92baee28bc39760c897f07dda999b", // norton
    "66441945b856ce8834470e3c616e3b0387acfc28619cf376fba9e2cdc209c6d0", // norton
    "6c00b5aa9741105d701e51985fe341d804b1623cd12814493e5c01c2582ae6b2", // norton
    "6f3231d9e188410c484449007f2329702b03c5c753270667d9c8c56c8da74eb2", // norton
    "711be395126bdf3215ce0fa7fc52865536386becef7304710e83f476a7c8852c", // norton
    "865aa245465155cde8f74f9eaf38bacb46e43faf41bcf93fe5e0bf35158a4148", // norton
    "86c2f4365f4fe1eb8423b728f24f35245a3d33d7677c0363f65a54716737947f", // norton
    "8de1d5edf5a9827c99fa4bb81f2348c12d47b3d6b6679d01337149d986368278", // norton
    "a1068a3ce24b7b2076fc23031d629b5d62ef767d3304302895949e813ee7d752", // norton
    "a870b7d13c79ee93e831bdcf773507631ece7d4ad962b5f86bb6bc00f369348a", // norton
    "f2a31db0158f66b17c3f219030f8b582c14b1c16e18b4982d89841237642c724", // norton
    "f642dd9b17f176057aa21d5f3295db8238f6105b1021b070560cfe99a4496e11", // norton
    "0c67489693478dad45cc540abfbd21adda5917c1ab4d5ca70eb37b1db8adfe17", // vscode
    "1a52318d1870370e11de7a8be56f4efc9e5fe63baa23f930fc1a0240dffe63df", // vscode
    "3bea7557d7ee3c29c8b977a5cb2c4fd891c688ad80f49b9e0ccb57bf3d0bd617", // vscode
    "5b0f942a395d915d4c0b8f9a3a063d33de82d27a7b099717f40b4723fa40cfd7", // vscode
    "61d59bf665062c30d14a8f01a8d9c2f3ac1cea37efaf264066ea0b347f891f38", // vscode
    "662224ab59469b53a3657cdfef43b99d254f13aad429dcb70f23003ebf43487c", // vscode
    "69404d73309ff849ae6945393cddefc3b8a3db2d94df82f18d1aa76ead79d526", // vscode
    "72665ab2022b75cc4201673fead866096e52ef7e628d152ae3f53a087151ecce", // vscode
    "73f32e4a88b277233aecc2a2618f84290023d48543dcac7e547d14b995c07e96", // vscode
    "803d1691d4c2fb03f072e0f67a5fe64feac3df4b3266a5c37ba4a4ec23cccda4", // vscode
    "89d89a1ff3089ee870db3a91b490ee6f6803646f3bb0285871f808e80106bfa1", // vscode
    "b56a37440729c9c443c0b8bebbaabaf7277e2263b820b69dd5655c48722bb541", // vscode
    "cae41b2a732674dd6d680e22614d83f2afc7cd7be0a433ac425a2f4ec1ed0cce", // vscode
    "e73ccfcb2e03da26abf46504568919a7f5d2c14fda4a85d4613eadd057248729", // vscode
    "eeb1130bacba97dac1a072145e628a3d06a641a0140b920f5edaa58994ba1e91", // vscode
    "f6f5ebdfe2e46438a90bd28226d2ef9d858138c3611bf55326d8907240fb7787", // vscode
    "2860429eb61001c0ea9572b28a7cf535c52e0c021d5fe6895d73beada8b0064c", // norton
    "db9adabae89079dcdc15cec6be0f27220af959e60a8e7e573febc1ecaf0ee28d", // standard
    "0a2c59ab2abdfecc434949ff74081db72763b3e1e9a42e83e2395eecb3a493a7", // neovim
    "4cdfdf01ce68483d823774c8d5b187547e76f61363f1b5ee8062fba4699e3806", // yazi
    "f23a6501a8a597627181c7845ef9e574cf52629555df75756b9fca8d6a8b5fb9", // norton
];

/// `true` when `digest` is the hash of a preset version Pairee shipped.
pub fn was_shipped(digest: &str) -> bool {
    SHIPPED.contains(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::embedded::PRESETS;

    #[test]
    fn every_current_preset_is_recorded() {
        let missing: Vec<String> = PRESETS
            .iter()
            .map(|(name, text)| (name, super::super::digest(text)))
            .filter(|(_, digest)| !was_shipped(digest))
            .map(|(name, digest)| format!("    \"{digest}\", // {name}"))
            .collect();
        assert!(
            missing.is_empty(),
            "add to SHIPPED:
{}",
            missing.join(
                "
"
            )
        );
    }
}
