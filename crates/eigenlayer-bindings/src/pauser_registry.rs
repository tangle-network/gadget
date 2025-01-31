#![allow(clippy::all, warnings)]

#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style
)]
pub mod PauserRegistry {
    use super::*;
    use alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x608060405234801561000f575f5ffd5b5060405161073638038061073683398101604081905261002e91610253565b5f5b825181101561006b5761006383828151811061004e5761004e61032f565b6020026020010151600161007c60201b60201c565b600101610030565b506100758161014d565b5050610343565b6001600160a01b0382166100ed5760405162461bcd60e51b815260206004820152602d60248201527f50617573657252656769737472792e5f7365745061757365723a207a65726f2060448201526c1859191c995cdcc81a5b9c1d5d609a1b60648201526084015b60405180910390fd5b6001600160a01b0382165f8181526020818152604091829020805460ff19168515159081179091558251938452908301527f65d3a1fd4c13f05cba164f80d03ce90fb4b5e21946bfc3ab7dbd434c2d0b9152910160405180910390a15050565b6001600160a01b0381166101bb5760405162461bcd60e51b815260206004820152602f60248201527f50617573657252656769737472792e5f736574556e7061757365723a207a657260448201526e1bc81859191c995cdcc81a5b9c1d5d608a1b60648201526084016100e4565b600154604080516001600160a01b03928316815291831660208301527f06b4167a2528887a1e97a366eefe8549bfbf1ea3e6ac81cb2564a934d20e8892910160405180910390a1600180546001600160a01b0319166001600160a01b0392909216919091179055565b634e487b7160e01b5f52604160045260245ffd5b80516001600160a01b038116811461024e575f5ffd5b919050565b5f5f60408385031215610264575f5ffd5b82516001600160401b03811115610279575f5ffd5b8301601f81018513610289575f5ffd5b80516001600160401b038111156102a2576102a2610224565b604051600582901b90603f8201601f191681016001600160401b03811182821017156102d0576102d0610224565b6040529182526020818401810192908101888411156102ed575f5ffd5b6020850194505b838510156103135761030585610238565b8152602094850194016102f4565b5094506103269250505060208401610238565b90509250929050565b634e487b7160e01b5f52603260045260245ffd5b6103e6806103505f395ff3fe608060405234801561000f575f5ffd5b506004361061004a575f3560e01c806346fbf68e1461004e5780638568520614610085578063ce5484281461009a578063eab66d7a146100ad575b5f5ffd5b61007061005c36600461030d565b5f6020819052908152604090205460ff1681565b60405190151581526020015b60405180910390f35b61009861009336600461032d565b6100d8565b005b6100986100a836600461030d565b610119565b6001546100c0906001600160a01b031681565b6040516001600160a01b03909116815260200161007c565b6001546001600160a01b0316331461010b5760405162461bcd60e51b815260040161010290610366565b60405180910390fd5b610115828261014f565b5050565b6001546001600160a01b031633146101435760405162461bcd60e51b815260040161010290610366565b61014c8161021b565b50565b6001600160a01b0382166101bb5760405162461bcd60e51b815260206004820152602d60248201527f50617573657252656769737472792e5f7365745061757365723a207a65726f2060448201526c1859191c995cdcc81a5b9c1d5d609a1b6064820152608401610102565b6001600160a01b0382165f8181526020818152604091829020805460ff19168515159081179091558251938452908301527f65d3a1fd4c13f05cba164f80d03ce90fb4b5e21946bfc3ab7dbd434c2d0b9152910160405180910390a15050565b6001600160a01b0381166102895760405162461bcd60e51b815260206004820152602f60248201527f50617573657252656769737472792e5f736574556e7061757365723a207a657260448201526e1bc81859191c995cdcc81a5b9c1d5d608a1b6064820152608401610102565b600154604080516001600160a01b03928316815291831660208301527f06b4167a2528887a1e97a366eefe8549bfbf1ea3e6ac81cb2564a934d20e8892910160405180910390a1600180546001600160a01b0319166001600160a01b0392909216919091179055565b80356001600160a01b0381168114610308575f5ffd5b919050565b5f6020828403121561031d575f5ffd5b610326826102f2565b9392505050565b5f5f6040838503121561033e575f5ffd5b610347836102f2565b91506020830135801515811461035b575f5ffd5b809150509250929050565b6020808252602a908201527f6d73672e73656e646572206973206e6f74207065726d697373696f6e6564206160408201526939903ab73830bab9b2b960b11b60608201526080019056fea2646970667358221220bad9bc7e5840c034ce8eb3bf0529db3b114379ed30673ef2860d67336dc90c2e64736f6c634300081b0033
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`@Qa\x0768\x03\x80a\x076\x839\x81\x01`@\x81\x90Ra\0.\x91a\x02SV[_[\x82Q\x81\x10\x15a\0kWa\0c\x83\x82\x81Q\x81\x10a\0NWa\0Na\x03/V[` \x02` \x01\x01Q`\x01a\0|` \x1B` \x1CV[`\x01\x01a\x000V[Pa\0u\x81a\x01MV[PPa\x03CV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\0\xEDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`-`$\x82\x01R\x7FPauserRegistry._setPauser: zero `D\x82\x01Rl\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x9A\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R` \x81\x81R`@\x91\x82\x90 \x80T`\xFF\x19\x16\x85\x15\x15\x90\x81\x17\x90\x91U\x82Q\x93\x84R\x90\x83\x01R\x7Fe\xD3\xA1\xFDL\x13\xF0\\\xBA\x16O\x80\xD0<\xE9\x0F\xB4\xB5\xE2\x19F\xBF\xC3\xAB}\xBDCL-\x0B\x91R\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x01\xBBW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`/`$\x82\x01R\x7FPauserRegistry._setUnpauser: zer`D\x82\x01Rn\x1B\xC8\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x8A\x1B`d\x82\x01R`\x84\x01a\0\xE4V[`\x01T`@\x80Q`\x01`\x01`\xA0\x1B\x03\x92\x83\x16\x81R\x91\x83\x16` \x83\x01R\x7F\x06\xB4\x16z%(\x88z\x1E\x97\xA3f\xEE\xFE\x85I\xBF\xBF\x1E\xA3\xE6\xAC\x81\xCB%d\xA94\xD2\x0E\x88\x92\x91\x01`@Q\x80\x91\x03\x90\xA1`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02NW__\xFD[\x91\x90PV[__`@\x83\x85\x03\x12\x15a\x02dW__\xFD[\x82Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02yW__\xFD[\x83\x01`\x1F\x81\x01\x85\x13a\x02\x89W__\xFD[\x80Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\xA2Wa\x02\xA2a\x02$V[`@Q`\x05\x82\x90\x1B\x90`?\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x02\xD0Wa\x02\xD0a\x02$V[`@R\x91\x82R` \x81\x84\x01\x81\x01\x92\x90\x81\x01\x88\x84\x11\x15a\x02\xEDW__\xFD[` \x85\x01\x94P[\x83\x85\x10\x15a\x03\x13Wa\x03\x05\x85a\x028V[\x81R` \x94\x85\x01\x94\x01a\x02\xF4V[P\x94Pa\x03&\x92PPP` \x84\x01a\x028V[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[a\x03\xE6\x80a\x03P_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0JW_5`\xE0\x1C\x80cF\xFB\xF6\x8E\x14a\0NW\x80c\x85hR\x06\x14a\0\x85W\x80c\xCET\x84(\x14a\0\x9AW\x80c\xEA\xB6mz\x14a\0\xADW[__\xFD[a\0pa\0\\6`\x04a\x03\rV[_` \x81\x90R\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\x98a\0\x936`\x04a\x03-V[a\0\xD8V[\0[a\0\x98a\0\xA86`\x04a\x03\rV[a\x01\x19V[`\x01Ta\0\xC0\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0|V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\x0BW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01\x02\x90a\x03fV[`@Q\x80\x91\x03\x90\xFD[a\x01\x15\x82\x82a\x01OV[PPV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01CW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01\x02\x90a\x03fV[a\x01L\x81a\x02\x1BV[PV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x01\xBBW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`-`$\x82\x01R\x7FPauserRegistry._setPauser: zero `D\x82\x01Rl\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x9A\x1B`d\x82\x01R`\x84\x01a\x01\x02V[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R` \x81\x81R`@\x91\x82\x90 \x80T`\xFF\x19\x16\x85\x15\x15\x90\x81\x17\x90\x91U\x82Q\x93\x84R\x90\x83\x01R\x7Fe\xD3\xA1\xFDL\x13\xF0\\\xBA\x16O\x80\xD0<\xE9\x0F\xB4\xB5\xE2\x19F\xBF\xC3\xAB}\xBDCL-\x0B\x91R\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x02\x89W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`/`$\x82\x01R\x7FPauserRegistry._setUnpauser: zer`D\x82\x01Rn\x1B\xC8\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x8A\x1B`d\x82\x01R`\x84\x01a\x01\x02V[`\x01T`@\x80Q`\x01`\x01`\xA0\x1B\x03\x92\x83\x16\x81R\x91\x83\x16` \x83\x01R\x7F\x06\xB4\x16z%(\x88z\x1E\x97\xA3f\xEE\xFE\x85I\xBF\xBF\x1E\xA3\xE6\xAC\x81\xCB%d\xA94\xD2\x0E\x88\x92\x91\x01`@Q\x80\x91\x03\x90\xA1`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x03\x08W__\xFD[\x91\x90PV[_` \x82\x84\x03\x12\x15a\x03\x1DW__\xFD[a\x03&\x82a\x02\xF2V[\x93\x92PPPV[__`@\x83\x85\x03\x12\x15a\x03>W__\xFD[a\x03G\x83a\x02\xF2V[\x91P` \x83\x015\x80\x15\x15\x81\x14a\x03[W__\xFD[\x80\x91PP\x92P\x92\x90PV[` \x80\x82R`*\x90\x82\x01R\x7Fmsg.sender is not permissioned a`@\x82\x01Ri9\x90:\xB780\xBA\xB9\xB2\xB9`\xB1\x1B``\x82\x01R`\x80\x01\x90V\xFE\xA2dipfsX\"\x12 \xBA\xD9\xBC~X@\xC04\xCE\x8E\xB3\xBF\x05)\xDB;\x11Cy\xED0g>\xF2\x86\rg3m\xC9\x0C.dsolcC\0\x08\x1B\x003",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405234801561000f575f5ffd5b506004361061004a575f3560e01c806346fbf68e1461004e5780638568520614610085578063ce5484281461009a578063eab66d7a146100ad575b5f5ffd5b61007061005c36600461030d565b5f6020819052908152604090205460ff1681565b60405190151581526020015b60405180910390f35b61009861009336600461032d565b6100d8565b005b6100986100a836600461030d565b610119565b6001546100c0906001600160a01b031681565b6040516001600160a01b03909116815260200161007c565b6001546001600160a01b0316331461010b5760405162461bcd60e51b815260040161010290610366565b60405180910390fd5b610115828261014f565b5050565b6001546001600160a01b031633146101435760405162461bcd60e51b815260040161010290610366565b61014c8161021b565b50565b6001600160a01b0382166101bb5760405162461bcd60e51b815260206004820152602d60248201527f50617573657252656769737472792e5f7365745061757365723a207a65726f2060448201526c1859191c995cdcc81a5b9c1d5d609a1b6064820152608401610102565b6001600160a01b0382165f8181526020818152604091829020805460ff19168515159081179091558251938452908301527f65d3a1fd4c13f05cba164f80d03ce90fb4b5e21946bfc3ab7dbd434c2d0b9152910160405180910390a15050565b6001600160a01b0381166102895760405162461bcd60e51b815260206004820152602f60248201527f50617573657252656769737472792e5f736574556e7061757365723a207a657260448201526e1bc81859191c995cdcc81a5b9c1d5d608a1b6064820152608401610102565b600154604080516001600160a01b03928316815291831660208301527f06b4167a2528887a1e97a366eefe8549bfbf1ea3e6ac81cb2564a934d20e8892910160405180910390a1600180546001600160a01b0319166001600160a01b0392909216919091179055565b80356001600160a01b0381168114610308575f5ffd5b919050565b5f6020828403121561031d575f5ffd5b610326826102f2565b9392505050565b5f5f6040838503121561033e575f5ffd5b610347836102f2565b91506020830135801515811461035b575f5ffd5b809150509250929050565b6020808252602a908201527f6d73672e73656e646572206973206e6f74207065726d697373696f6e6564206160408201526939903ab73830bab9b2b960b11b60608201526080019056fea2646970667358221220bad9bc7e5840c034ce8eb3bf0529db3b114379ed30673ef2860d67336dc90c2e64736f6c634300081b0033
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0JW_5`\xE0\x1C\x80cF\xFB\xF6\x8E\x14a\0NW\x80c\x85hR\x06\x14a\0\x85W\x80c\xCET\x84(\x14a\0\x9AW\x80c\xEA\xB6mz\x14a\0\xADW[__\xFD[a\0pa\0\\6`\x04a\x03\rV[_` \x81\x90R\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\x98a\0\x936`\x04a\x03-V[a\0\xD8V[\0[a\0\x98a\0\xA86`\x04a\x03\rV[a\x01\x19V[`\x01Ta\0\xC0\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0|V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\x0BW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01\x02\x90a\x03fV[`@Q\x80\x91\x03\x90\xFD[a\x01\x15\x82\x82a\x01OV[PPV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01CW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01\x02\x90a\x03fV[a\x01L\x81a\x02\x1BV[PV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x01\xBBW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`-`$\x82\x01R\x7FPauserRegistry._setPauser: zero `D\x82\x01Rl\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x9A\x1B`d\x82\x01R`\x84\x01a\x01\x02V[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R` \x81\x81R`@\x91\x82\x90 \x80T`\xFF\x19\x16\x85\x15\x15\x90\x81\x17\x90\x91U\x82Q\x93\x84R\x90\x83\x01R\x7Fe\xD3\xA1\xFDL\x13\xF0\\\xBA\x16O\x80\xD0<\xE9\x0F\xB4\xB5\xE2\x19F\xBF\xC3\xAB}\xBDCL-\x0B\x91R\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x02\x89W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`/`$\x82\x01R\x7FPauserRegistry._setUnpauser: zer`D\x82\x01Rn\x1B\xC8\x18Y\x19\x1C\x99\\\xDC\xC8\x1A[\x9C\x1D]`\x8A\x1B`d\x82\x01R`\x84\x01a\x01\x02V[`\x01T`@\x80Q`\x01`\x01`\xA0\x1B\x03\x92\x83\x16\x81R\x91\x83\x16` \x83\x01R\x7F\x06\xB4\x16z%(\x88z\x1E\x97\xA3f\xEE\xFE\x85I\xBF\xBF\x1E\xA3\xE6\xAC\x81\xCB%d\xA94\xD2\x0E\x88\x92\x91\x01`@Q\x80\x91\x03\x90\xA1`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x03\x08W__\xFD[\x91\x90PV[_` \x82\x84\x03\x12\x15a\x03\x1DW__\xFD[a\x03&\x82a\x02\xF2V[\x93\x92PPPV[__`@\x83\x85\x03\x12\x15a\x03>W__\xFD[a\x03G\x83a\x02\xF2V[\x91P` \x83\x015\x80\x15\x15\x81\x14a\x03[W__\xFD[\x80\x91PP\x92P\x92\x90PV[` \x80\x82R`*\x90\x82\x01R\x7Fmsg.sender is not permissioned a`@\x82\x01Ri9\x90:\xB780\xBA\xB9\xB2\xB9`\xB1\x1B``\x82\x01R`\x80\x01\x90V\xFE\xA2dipfsX\"\x12 \xBA\xD9\xBC~X@\xC04\xCE\x8E\xB3\xBF\x05)\xDB;\x11Cy\xED0g>\xF2\x86\rg3m\xC9\x0C.dsolcC\0\x08\x1B\x003",
    );
    /**Event with signature `PauserStatusChanged(address,bool)` and selector `0x65d3a1fd4c13f05cba164f80d03ce90fb4b5e21946bfc3ab7dbd434c2d0b9152`.
    ```solidity
    event PauserStatusChanged(address pauser, bool canPause);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct PauserStatusChanged {
        #[allow(missing_docs)]
        pub pauser: alloy_sol_types::private::Address,
        #[allow(missing_docs)]
        pub canPause: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for PauserStatusChanged {
            type DataTuple<'a> = (
                alloy_sol_types::sol_data::Address,
                alloy_sol_types::sol_data::Bool,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "PauserStatusChanged(address,bool)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    101u8, 211u8, 161u8, 253u8, 76u8, 19u8, 240u8, 92u8, 186u8, 22u8, 79u8, 128u8,
                    208u8, 60u8, 233u8, 15u8, 180u8, 181u8, 226u8, 25u8, 70u8, 191u8, 195u8, 171u8,
                    125u8, 189u8, 67u8, 76u8, 45u8, 11u8, 145u8, 82u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    pauser: data.0,
                    canPause: data.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.pauser,
                    ),
                    <alloy_sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(
                        &self.canPause,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for PauserStatusChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&PauserStatusChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &PauserStatusChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Event with signature `UnpauserChanged(address,address)` and selector `0x06b4167a2528887a1e97a366eefe8549bfbf1ea3e6ac81cb2564a934d20e8892`.
    ```solidity
    event UnpauserChanged(address previousUnpauser, address newUnpauser);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct UnpauserChanged {
        #[allow(missing_docs)]
        pub previousUnpauser: alloy_sol_types::private::Address,
        #[allow(missing_docs)]
        pub newUnpauser: alloy_sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for UnpauserChanged {
            type DataTuple<'a> = (
                alloy_sol_types::sol_data::Address,
                alloy_sol_types::sol_data::Address,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "UnpauserChanged(address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    6u8, 180u8, 22u8, 122u8, 37u8, 40u8, 136u8, 122u8, 30u8, 151u8, 163u8, 102u8,
                    238u8, 254u8, 133u8, 73u8, 191u8, 191u8, 30u8, 163u8, 230u8, 172u8, 129u8,
                    203u8, 37u8, 100u8, 169u8, 52u8, 210u8, 14u8, 136u8, 146u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    previousUnpauser: data.0,
                    newUnpauser: data.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.previousUnpauser,
                    ),
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.newUnpauser,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for UnpauserChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&UnpauserChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &UnpauserChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Constructor`.
    ```solidity
    constructor(address[] _pausers, address _unpauser);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct constructorCall {
        pub _pausers: alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
        pub _unpauser: alloy_sol_types::private::Address,
    }
    const _: () = {
        use alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy_sol_types::sol_data::Array<alloy_sol_types::sol_data::Address>,
                alloy_sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
                alloy_sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<constructorCall> for UnderlyingRustTuple<'_> {
                fn from(value: constructorCall) -> Self {
                    (value._pausers, value._unpauser)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for constructorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _pausers: tuple.0,
                        _unpauser: tuple.1,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolConstructor for constructorCall {
            type Parameters<'a> = (
                alloy_sol_types::sol_data::Array<alloy_sol_types::sol_data::Address>,
                alloy_sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy_sol_types::sol_data::Array<
                        alloy_sol_types::sol_data::Address,
                    > as alloy_sol_types::SolType>::tokenize(&self._pausers),
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._unpauser,
                    ),
                )
            }
        }
    };
    /**Function with signature `isPauser(address)` and selector `0x46fbf68e`.
    ```solidity
    function isPauser(address) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPauserCall {
        pub _0: alloy_sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`isPauser(address)`](isPauserCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPauserReturn {
        pub _0: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy_sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy_sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<isPauserCall> for UnderlyingRustTuple<'_> {
                fn from(value: isPauserCall) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isPauserCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy_sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<isPauserReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isPauserReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isPauserReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isPauserCall {
            type Parameters<'a> = (alloy_sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isPauserReturn;
            type ReturnTuple<'a> = (alloy_sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isPauser(address)";
            const SELECTOR: [u8; 4] = [70u8, 251u8, 246u8, 142u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._0,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `setIsPauser(address,bool)` and selector `0x85685206`.
    ```solidity
    function setIsPauser(address newPauser, bool canPause) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setIsPauserCall {
        pub newPauser: alloy_sol_types::private::Address,
        pub canPause: bool,
    }
    ///Container type for the return parameters of the [`setIsPauser(address,bool)`](setIsPauserCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setIsPauserReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy_sol_types::sol_data::Address,
                alloy_sol_types::sol_data::Bool,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy_sol_types::private::Address, bool);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<setIsPauserCall> for UnderlyingRustTuple<'_> {
                fn from(value: setIsPauserCall) -> Self {
                    (value.newPauser, value.canPause)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setIsPauserCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newPauser: tuple.0,
                        canPause: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<setIsPauserReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setIsPauserReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setIsPauserReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setIsPauserCall {
            type Parameters<'a> = (
                alloy_sol_types::sol_data::Address,
                alloy_sol_types::sol_data::Bool,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = setIsPauserReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setIsPauser(address,bool)";
            const SELECTOR: [u8; 4] = [133u8, 104u8, 82u8, 6u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.newPauser,
                    ),
                    <alloy_sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(
                        &self.canPause,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `setUnpauser(address)` and selector `0xce548428`.
    ```solidity
    function setUnpauser(address newUnpauser) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setUnpauserCall {
        pub newUnpauser: alloy_sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`setUnpauser(address)`](setUnpauserCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setUnpauserReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy_sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy_sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<setUnpauserCall> for UnderlyingRustTuple<'_> {
                fn from(value: setUnpauserCall) -> Self {
                    (value.newUnpauser,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setUnpauserCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newUnpauser: tuple.0,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<setUnpauserReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setUnpauserReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setUnpauserReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setUnpauserCall {
            type Parameters<'a> = (alloy_sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = setUnpauserReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setUnpauser(address)";
            const SELECTOR: [u8; 4] = [206u8, 84u8, 132u8, 40u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy_sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.newUnpauser,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `unpauser()` and selector `0xeab66d7a`.
    ```solidity
    function unpauser() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct unpauserCall {}
    ///Container type for the return parameters of the [`unpauser()`](unpauserCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct unpauserReturn {
        pub _0: alloy_sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<unpauserCall> for UnderlyingRustTuple<'_> {
                fn from(value: unpauserCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for unpauserCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy_sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy_sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<unpauserReturn> for UnderlyingRustTuple<'_> {
                fn from(value: unpauserReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for unpauserReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for unpauserCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = unpauserReturn;
            type ReturnTuple<'a> = (alloy_sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "unpauser()";
            const SELECTOR: [u8; 4] = [234u8, 182u8, 109u8, 122u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    ///Container for all the [`PauserRegistry`](self) function calls.
    pub enum PauserRegistryCalls {
        isPauser(isPauserCall),
        setIsPauser(setIsPauserCall),
        setUnpauser(setUnpauserCall),
        unpauser(unpauserCall),
    }
    #[automatically_derived]
    impl PauserRegistryCalls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [70u8, 251u8, 246u8, 142u8],
            [133u8, 104u8, 82u8, 6u8],
            [206u8, 84u8, 132u8, 40u8],
            [234u8, 182u8, 109u8, 122u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PauserRegistryCalls {
        const NAME: &'static str = "PauserRegistryCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 4usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::isPauser(_) => <isPauserCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::setIsPauser(_) => <setIsPauserCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::setUnpauser(_) => <setUnpauserCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::unpauser(_) => <unpauserCall as alloy_sol_types::SolCall>::SELECTOR,
            }
        }
        #[inline]
        fn selector_at(i: usize) -> ::core::option::Option<[u8; 4]> {
            Self::SELECTORS.get(i).copied()
        }
        #[inline]
        fn valid_selector(selector: [u8; 4]) -> bool {
            Self::SELECTORS.binary_search(&selector).is_ok()
        }
        #[inline]
        #[allow(unsafe_code, non_snake_case)]
        fn abi_decode_raw(
            selector: [u8; 4],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
                bool,
            )
                -> alloy_sol_types::Result<PauserRegistryCalls>] = &[
                {
                    fn isPauser(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PauserRegistryCalls> {
                        <isPauserCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PauserRegistryCalls::isPauser)
                    }
                    isPauser
                },
                {
                    fn setIsPauser(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PauserRegistryCalls> {
                        <setIsPauserCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PauserRegistryCalls::setIsPauser)
                    }
                    setIsPauser
                },
                {
                    fn setUnpauser(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PauserRegistryCalls> {
                        <setUnpauserCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PauserRegistryCalls::setUnpauser)
                    }
                    setUnpauser
                },
                {
                    fn unpauser(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PauserRegistryCalls> {
                        <unpauserCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PauserRegistryCalls::unpauser)
                    }
                    unpauser
                },
            ];
            let Ok(idx) = Self::SELECTORS.binary_search(&selector) else {
                return Err(alloy_sol_types::Error::unknown_selector(
                    <Self as alloy_sol_types::SolInterface>::NAME,
                    selector,
                ));
            };
            (unsafe { DECODE_SHIMS.get_unchecked(idx) })(data, validate)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::isPauser(inner) => {
                    <isPauserCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::setIsPauser(inner) => {
                    <setIsPauserCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::setUnpauser(inner) => {
                    <setUnpauserCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::unpauser(inner) => {
                    <unpauserCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::isPauser(inner) => {
                    <isPauserCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::setIsPauser(inner) => {
                    <setIsPauserCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::setUnpauser(inner) => {
                    <setUnpauserCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::unpauser(inner) => {
                    <unpauserCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
            }
        }
    }
    ///Container for all the [`PauserRegistry`](self) events.
    pub enum PauserRegistryEvents {
        PauserStatusChanged(PauserStatusChanged),
        UnpauserChanged(UnpauserChanged),
    }
    #[automatically_derived]
    impl PauserRegistryEvents {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                6u8, 180u8, 22u8, 122u8, 37u8, 40u8, 136u8, 122u8, 30u8, 151u8, 163u8, 102u8,
                238u8, 254u8, 133u8, 73u8, 191u8, 191u8, 30u8, 163u8, 230u8, 172u8, 129u8, 203u8,
                37u8, 100u8, 169u8, 52u8, 210u8, 14u8, 136u8, 146u8,
            ],
            [
                101u8, 211u8, 161u8, 253u8, 76u8, 19u8, 240u8, 92u8, 186u8, 22u8, 79u8, 128u8,
                208u8, 60u8, 233u8, 15u8, 180u8, 181u8, 226u8, 25u8, 70u8, 191u8, 195u8, 171u8,
                125u8, 189u8, 67u8, 76u8, 45u8, 11u8, 145u8, 82u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for PauserRegistryEvents {
        const NAME: &'static str = "PauserRegistryEvents";
        const COUNT: usize = 2usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<PauserStatusChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <PauserStatusChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::PauserStatusChanged)
                }
                Some(<UnpauserChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <UnpauserChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::UnpauserChanged)
                }
                _ => alloy_sol_types::private::Err(alloy_sol_types::Error::InvalidLog {
                    name: <Self as alloy_sol_types::SolEventInterface>::NAME,
                    log: alloy_sol_types::private::Box::new(
                        alloy_sol_types::private::LogData::new_unchecked(
                            topics.to_vec(),
                            data.to_vec().into(),
                        ),
                    ),
                }),
            }
        }
    }
    #[automatically_derived]
    impl alloy_sol_types::private::IntoLogData for PauserRegistryEvents {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::PauserStatusChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::UnpauserChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::PauserStatusChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::UnpauserChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
            }
        }
    }
    use alloy_contract;
    /**Creates a new wrapper around an on-chain [`PauserRegistry`](self) contract instance.

    See the [wrapper's documentation](`PauserRegistryInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> PauserRegistryInstance<T, P, N> {
        PauserRegistryInstance::<T, P, N>::new(address, provider)
    }
    /**Deploys this contract using the given `provider` and constructor arguments, if any.

    Returns a new instance of the contract, if the deployment was successful.

    For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
    #[inline]
    pub fn deploy<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        provider: P,
        _pausers: alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
        _unpauser: alloy_sol_types::private::Address,
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<PauserRegistryInstance<T, P, N>>>
    {
        PauserRegistryInstance::<T, P, N>::deploy(provider, _pausers, _unpauser)
    }
    /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
    and constructor arguments, if any.

    This is a simple wrapper around creating a `RawCallBuilder` with the data set to
    the bytecode concatenated with the constructor's ABI-encoded arguments.*/
    #[inline]
    pub fn deploy_builder<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        provider: P,
        _pausers: alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
        _unpauser: alloy_sol_types::private::Address,
    ) -> alloy_contract::RawCallBuilder<T, P, N> {
        PauserRegistryInstance::<T, P, N>::deploy_builder(provider, _pausers, _unpauser)
    }
    /**A [`PauserRegistry`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`PauserRegistry`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct PauserRegistryInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for PauserRegistryInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("PauserRegistryInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > PauserRegistryInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`PauserRegistry`](self) contract instance.

        See the [wrapper's documentation](`PauserRegistryInstance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
            Self {
                address,
                provider,
                _network_transport: ::core::marker::PhantomData,
            }
        }
        /**Deploys this contract using the given `provider` and constructor arguments, if any.

        Returns a new instance of the contract, if the deployment was successful.

        For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
        #[inline]
        pub async fn deploy(
            provider: P,
            _pausers: alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
            _unpauser: alloy_sol_types::private::Address,
        ) -> alloy_contract::Result<PauserRegistryInstance<T, P, N>> {
            let call_builder = Self::deploy_builder(provider, _pausers, _unpauser);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
        and constructor arguments, if any.

        This is a simple wrapper around creating a `RawCallBuilder` with the data set to
        the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(
            provider: P,
            _pausers: alloy_sol_types::private::Vec<alloy_sol_types::private::Address>,
            _unpauser: alloy_sol_types::private::Address,
        ) -> alloy_contract::RawCallBuilder<T, P, N> {
            alloy_contract::RawCallBuilder::new_raw_deploy(
                provider,
                [
                    &BYTECODE[..],
                    &alloy_sol_types::SolConstructor::abi_encode(&constructorCall {
                        _pausers,
                        _unpauser,
                    })[..],
                ]
                .concat()
                .into(),
            )
        }
        /// Returns a reference to the address.
        #[inline]
        pub const fn address(&self) -> &alloy_sol_types::private::Address {
            &self.address
        }
        /// Sets the address.
        #[inline]
        pub fn set_address(&mut self, address: alloy_sol_types::private::Address) {
            self.address = address;
        }
        /// Sets the address and returns `self`.
        pub fn at(mut self, address: alloy_sol_types::private::Address) -> Self {
            self.set_address(address);
            self
        }
        /// Returns a reference to the provider.
        #[inline]
        pub const fn provider(&self) -> &P {
            &self.provider
        }
    }
    impl<T, P: ::core::clone::Clone, N> PauserRegistryInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> PauserRegistryInstance<T, P, N> {
            PauserRegistryInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network_transport: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > PauserRegistryInstance<T, P, N>
    {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<T, &P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
        ///Creates a new call builder for the [`isPauser`] function.
        pub fn isPauser(
            &self,
            _0: alloy_sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, isPauserCall, N> {
            self.call_builder(&isPauserCall { _0 })
        }
        ///Creates a new call builder for the [`setIsPauser`] function.
        pub fn setIsPauser(
            &self,
            newPauser: alloy_sol_types::private::Address,
            canPause: bool,
        ) -> alloy_contract::SolCallBuilder<T, &P, setIsPauserCall, N> {
            self.call_builder(&setIsPauserCall {
                newPauser,
                canPause,
            })
        }
        ///Creates a new call builder for the [`setUnpauser`] function.
        pub fn setUnpauser(
            &self,
            newUnpauser: alloy_sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, setUnpauserCall, N> {
            self.call_builder(&setUnpauserCall { newUnpauser })
        }
        ///Creates a new call builder for the [`unpauser`] function.
        pub fn unpauser(&self) -> alloy_contract::SolCallBuilder<T, &P, unpauserCall, N> {
            self.call_builder(&unpauserCall {})
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > PauserRegistryInstance<T, P, N>
    {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<T, &P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
        ///Creates a new event filter for the [`PauserStatusChanged`] event.
        pub fn PauserStatusChanged_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, PauserStatusChanged, N> {
            self.event_filter::<PauserStatusChanged>()
        }
        ///Creates a new event filter for the [`UnpauserChanged`] event.
        pub fn UnpauserChanged_filter(&self) -> alloy_contract::Event<T, &P, UnpauserChanged, N> {
            self.event_filter::<UnpauserChanged>()
        }
    }
}
