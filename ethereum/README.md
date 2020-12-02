# Ethereum Cash Token and Starport

* `yarn install`
* `yarn compile`
* `yarn test`


## Testnet Deploy
Deploy with a list of validators:
* `npx saddle script deploy -n goerli 0x513c1Ff435ECCEdD0fDA5edD2Ad5E5461F0E8726 0x513c1Ff435ECCEdD0fDA5edD2Ad5E5461F0E8726`

Check with
* `yarn console -n goerli`
	* `await starport.methods.cash().call();`

