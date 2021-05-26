const EC = require("elliptic").ec;
//const ec = new EC('secp256k1');
const ec = new EC("p256");
const { SHA3 } = require("sha3");

// Generate a new key pair and convert them to hex-strings
let keyPair = ec.keyFromPrivate(
  "97ddae0f3a25b92268175400149d65d6887b9cefaf28ea2c078e05cdc15a3c0a"
);
let privKey = keyPair.getPrivate("hex");
let pubKey = keyPair.getPublic();
console.log(`Private key: ${privKey}`);
console.log("Public key :", pubKey.encode("hex").substr(2));
console.log("Public key (compressed):", pubKey.encodeCompressed("hex"));

let msg = "foo";
let msgHex = "666f6f";
const sha = new SHA3(256);
sha.update(Buffer.from(msgHex, "hex"));
const msgHash = sha.digest();
//let msgHash = sha3.keccak256(msg);
const key = ec.keyFromPrivate(
  Buffer.from(
    "97ddae0f3a25b92268175400149d65d6887b9cefaf28ea2c078e05cdc15a3c0a",
    "hex"
  )
);
//let signature = ec.sign(msgHash, privKey, "hex", {canonical: true});
let signature = key.sign(`user:${msgHash}`);
console.log(`Msg: ${msg}`);
console.log(`Msg hash: ${msgHash}`);
console.log("Signature:", signature);

const n = 32; // half of signature length?
const r = signature.r.toArrayLike(Buffer, "be", n);
const s = signature.s.toArrayLike(Buffer, "be", n);
const res = Buffer.concat([r, s]).toString("hex");
console.log("signature res = ", res);

const signatureHex = signature.toDER("hex");
console.log("signature hex = ", signatureHex);

let hexToDecimal = (x) => ec.keyFromPrivate(x, "hex").getPrivate().toString(10);
let pubKeyRecovered = ec.recoverPubKey(
  hexToDecimal(msgHash),
  signature,
  signature.recoveryParam,
  "hex"
);
console.log("Recovered pubKey:", pubKeyRecovered.encodeCompressed("hex"));

let validSig = ec.verify(msgHash, signature, pubKeyRecovered);
console.log("Signature valid?", validSig);
