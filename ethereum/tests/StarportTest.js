describe('Starport', () => {
  let starport;

  beforeEach(async () => {
    starport = await deploy('Starport', []);
  });


  it('should foo', async () => {
    const res = await call(starport, 'foo',[4]);
    expect(res).numEquals('4');
  });

  it('should revert foo if not 4', async () => {
    await expect(call(starport, 'foo',[3])).rejects.toRevert("revert bar");
  });

});
