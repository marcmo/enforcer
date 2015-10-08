int gcd(int a, int b)
{
  	for (;;)
	{
		if (a == 0)
		{
			return b;
		}
        b %= a;
        if (b == 0)
        {
            return a;
        }
        a %= b;
    }
}

asm UINT32 read_MSR(void)
{
  mfmsr	r3
  wrteei 0
}
