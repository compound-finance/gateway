const { SHA3 } = require("sha3");
const EC = require("elliptic").ec;

const ec = new EC("secp256k1");

const hashMsgHex = (msgHex) => {
  const sha = new SHA3(256);
  sha.update(Buffer.from(msgHex, "hex"));
  return sha.digest();
};

const signWithKey = (privateKey, msgHex) => {
  const key = ec.keyFromPrivate(Buffer.from(privateKey, "hex"));
  const sig = key.sign(hashMsgHex(msgHex));
  console.log("sig = ", sig.toDER("hex"));
  const n = 64; // half of signature length?
  const r = sig.r.toArrayLike(Buffer, "be", n);
  const s = sig.s.toArrayLike(Buffer, "be", n);
  return Buffer.concat([r, s]).toString("hex");
};

function main() {
  const msg = "foo";
  const encoded = new Buffer(msg).toString("hex");
  console.log("encoded = ", encoded);

  // var key = ec.genKeyPair();
  // const signature = key.sign(hashMsgHex(encoded))
  // console.log("signature = ", signature.toString())

  // const privateKey = keyPair.getPrivate().toString();

  //console.log("key = ", key)

  // var key = ec.keyFromPublic(pub, 'hex');

  // var keyPair = ec.genKeyPair();
  // const publicKey_x = key.getPublic().getX().toString();
  // const publicKey_y = key.getPublic().getY().toString();

  // console.log("publicKey_x = ", publicKey_x)
  // console.log("publicKey_y = ", publicKey_y)

  // const privateKey = key.getPrivate().toString();
  // // // console.log("publicKey = ", publicKey)
  // console.log("privateKey = ", privateKey)

  //const signature = signWithKey("8b8616eb5cc91cb453b4cdf057f4f9b6fe8210832866177a98f4cc9469ccecbe ", encoded)
  const signature = signWithKey(
    "d88a39391606b80299f85aecce1f5ba126ac135bbe068529ff42f0514ce80e66",
    encoded
  );
  console.log("signature = ", signature);
}

main();
