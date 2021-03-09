
 - Run Scripts
    CashToken:
     - runCashToken.sh: specs without any summarizing
     - runCashHarness.sh: spects summarizing index as 1e18 and axiomatizing
                          amountToPrincipal as monotonic
    Starport:
     - runStarportHarness.sh: specs, no summaries but harnessing needed
     - runStarportHarnessOrdering.sh:  partial ordering spec, override invokeNoticeInternal to not
                                       perform the function call since this havocs all storage