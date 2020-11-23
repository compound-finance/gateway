describe('CashToken', () => {
  let cashToken;

  beforeEach(async () => {
    cashToken = await deploy('CashToken', []);
  });


  it('should foo', async () => {
    const res = await call(cashToken, 'foo',[4]);
    expect(res).numEquals('4');
  });

  it('should revert foo if not 4', async () => {
    await expect(call(cashToken, 'foo',[3])).rejects.toRevert("revert bar");
  });

});
