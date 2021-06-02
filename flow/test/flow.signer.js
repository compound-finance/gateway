const EC = require("elliptic").ec;
//const ec = new EC('secp256k1');
const ec = new EC("p256");
const { SHA3 } = require("sha3");

const FLOW_TAG = [70, 76, 79, 87, 45, 86, 48, 46, 48, 45, 117, 115, 101, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

const hashMsg = (msgBytes) => {
    const sha = new SHA3(256);
    sha.update(Buffer.from(msgBytes));
    return sha.digest();
};

export const signWithKey = (privateKey, message) => {
    const key = ec.keyFromPrivate(Buffer.from(privateKey, "hex"));
    const messageWithTag = FLOW_TAG.concat(message)
    const sig = key.sign(hashMsg(messageWithTag));
    const n = 32;
    const r = sig.r.toArrayLike(Buffer, "be", n);
    const s = sig.s.toArrayLike(Buffer, "be", n);
    return Buffer.concat([r, s]).toString("hex");
};
