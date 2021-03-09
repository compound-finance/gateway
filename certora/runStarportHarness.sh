certoraRun.py contracts/StarportHarness.sol \
	spec/harnesses/ERC20.sol \
	spec/harnesses/NonStandardERC20.sol \
	spec/harnesses/ExcessiveERC20.sol \
	contracts/CashToken.sol \
	--verify StarportHarness:spec/Starport.spec \
	--solc solc8.1 --settings -t=300,-assumeUnwindCond,-b=3 \
	--cache "the-starport-cache" \
	--cloud --msg "Starport with Harness"