pragma circom 2.1.6;

include "../../node_modules/circomlib/circuits/sha256/sha256_2.circom";

template Hash(){
  signal input a;
  signal input b;
  signal output out;
  component sha256_2 = Sha256_2();
  sha256_2.a <== a;
  sha256_2.b <== b;
  out <== sha256_2.out;
}

component main = Hash();

