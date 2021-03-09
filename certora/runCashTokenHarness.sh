certoraRun.py contracts/CashTokenHarness.sol \
	--verify CashTokenHarness:spec/CashTokenHarness.spec \
	--solc solc8.1 --settings -t=300,-ignoreViewFunctions,-assumeUnwindCond,-b=3 \
	--cache "CashTokenHarness" \
	--cloud --msg "CashToken with Harness: ERC20 etc."