async function loadem(starting_page = 0) {
    var cash_nonce = await api.query.cash.nonces({eth: saddle.account});
    await feedAccount(cash_nonce, starting_page);
}

async function feedAccount(n, r, accounts = [] ) {
    const user = saddle.account
    var accounts
    var cash_nonce = n
    if (r > 2000) return;

    if (accounts.length == 0 || i == accounts.length) {
        console.log(r)
        accounts =  
            await fetch(`https://api.compound.finance/api/v2/account?page_number=${r}&page_size=200`)
                .then(r => r.json())
                .then( b => b.accounts.map( x => x.address))
        i = 0;
        r++;
    }

    console.log(accounts)
    for (const a of accounts) {
        // let a = accounts[i]
        let request = `(Transfer ${1234567 + cash_nonce} Cash Eth:${a})`
        let req = `${cash_nonce}:${request}`
        let sig = await saddle.web3.eth.sign(req, user)
        console.log("🅰️", a)
        console.log("🎲", req)
        let tx = api.tx.cash.execTrxRequest(request, {'Eth': [user, sig]}, cash_nonce)

        await tx.send(({ status }) => {
            if (status.isDropped) {
                console.log(`dopped ${status.events}`);
                }
            if (status.isInvalid) {
                console.log(`invalid ${status.events}`);
                }
            if (status.isInBlock) {
                console.log(`included in ${status}`);
            
            }
            if (status.isFinalized) {
                console.log(`${request} finailzied in ${status}`);
            }
        })
        cash_nonce++
    }
    feedAccount(cash_nonce, r)
}