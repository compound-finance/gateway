certoraRun.py contracts/StarportHarnessOrdering.sol \
	spec/harnesses/ERC20.sol \
	spec/harnesses/NonStandardERC20.sol \
	spec/harnesses/ExcessiveERC20.sol \
	contracts/CashToken.sol \
	--verify StarportHarnessOrdering:spec/StarportOrdering.spec \
	--solc solc8.1 --settings -t=300,-assumeUnwindCond,-b=3 \
	--cache "starport-ordering" \
	--cloud --msg "Starport with Harness: Notice Partial Ordering"