QSS	QSB P.260	hour	13	Matrix
Component Name	Type (C/S/Q/G)	Immediate Follower (Name / Prob / TransferTime, separated by ',')	Input Rule	Output Rule	Queue Discipline	Queue Capacity	Attribute Value	Interarrival Time Distribution	Batch Size Distribution	Service Time Distribution
A	C	BUFFER 1	 	 	 	 	 	EXP/0/2	CONSTANT/1	
B	C	BUFFER 3	 	 	 	 	 	NORMAL/2/1	CONSTANT/1	
Station 1	S	BUFFER 2//.2	 	 	 	 	 	 	 	A/EXP//1.8
Station 2	S	BUFFER 5//.2	 	 	 	 	 	 	 	A/EXP/1/1
Station 3	S	BUFFER 4//.1	 	 	 	 	 	 	 	B/UNIFORM/1.5/2
Station 4	S	BUFFER 6//.3	 	 	 	 	 	 	 	B/CONST/1.5
Station 5	S	 	Assembly	 	 	 	 	 	 	A/TRI/1/1.5/2,B/TRI/1/1.5/2
Buffer 1	Q	STATION 1	 	 	FIFO	50	 	 	 	
Buffer 2	Q	STATION 2	 	 	FIFO	50	 	 	 	
Buffer 3	Q	STATION 3	 	 	FIFO	50	 	 	 	
Buffer 4	Q	STATION 4	 	 	FIFO	50	 	 	 	
Buffer 5	Q	STATION 5	 	 	FIFO	50	 	 	 	
Buffer 6	Q	STATION 5	 	 	FIFO	50	 	 	 	
0
