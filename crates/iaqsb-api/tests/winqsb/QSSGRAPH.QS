QSS	QSB P.260	hour	13	Graphic
Component Name	Component Type	Immediate Follower	Input Rule	Output Rule	Queue Discipline	Queue Capacity	Attribute Value	Interarrival Time Distribution	Batch Size Distribution	Service Time Distribution
A	C	Buffer 1						Exp//2	Cons/1	
B	C	Buffer 3						Normal/2/1	const/1	
Buffer 1	Q	Station 1				50				
Buffer 3	Q	Station 3				50				
Station 1	S	Buffer 2//.2								A/Exp//1.8
Station 3	S	Buffer 4//.1								B/Uniform/1.5/2
Buffer 2	Q	Station 2				50				
Buffer 4	Q	Station 4				50				
Station 2	S	Buffer 5//.2								A/Exp/1/1
Station 4	S	Buffer 6//.3								B/const/1.5
Buffer 5	Q	Station 5				50				
Buffer 6	Q	Station 5				50				
Station 5	S		Assembly							A/Tri/1/1.5/2,B/Tri/1/1.5/2
-1
1	1	1
2	3	1
3	1	2
4	3	2
5	1	3
6	3	3
7	1	4
8	3	4
9	1	5
10	3	5
11	1	6
12	3	6
13	2	7
