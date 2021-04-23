certoraRun.py contracts/CashToken.sol \
	--verify CashToken:spec/CashToken.spec \
	--solc solc8.1 --settings -t=300,-assumeUnwindCond,-b=3 \
	--cache "CashToken" \
	--cloud --msg "CashToken: ERC20 etc."