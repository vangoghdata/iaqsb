QSS	QSB P.252	hour	4	Matrix
Component Name	Type (C/S/Q/G)	Immediate Follower (Name / Prob / TransferTime, separated by ',')	Input Rule	Output Rule	Queue Discipline	Queue Capacity	Attribute Value	Interarrival Time Distribution	Batch Size Distribution	Service Time Distribution
Clerk John	S	 	 	 	 	 	 	 	 	Customer/Exp//1.7142857
Clerk Mary	S	 	 	 	 	 	 	 	 	Customer/Exp//2
Customer	C	Queue	 	 	 	 	 	Exp/0/1.2	Constant/1	
Queue	Q	Clerk John,Clerk Mary	 	 	FIFO	100	 	 	 	
0
