methods {
	0xee872558 /*executeOperation(address,uint256,uint256,bytes memory)*/ => HAVOC_ALL;
}
rule sanity(method f) {
	env e;
	calldataarg arg;
	sinvoke f(e, arg);
	assert false;
}