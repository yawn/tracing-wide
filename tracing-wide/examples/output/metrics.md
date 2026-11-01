```console
$ cargo run -q --example metrics --features sink
Metrics
Resource
...
	Instrumentation Scope #0
		Name         : tracing-wide-example
Metric #0
		Name         : queue.depth
		Description  : 
		Unit         : 
		Type         : Gauge
		Gauge DataPoints
		StartTime    : [..]
		EndTime      : [..]
		DataPoint #0
			Value        : 3
			Attributes   :
Metric #1
		Name         : request.duration
		Description  : 
		Unit         : ms
		Type         : Histogram
		Temporality  : Cumulative
		StartTime    : [..]
		EndTime      : [..]
		Histogram DataPoints
		DataPoint #0
			Count        : 1
			Sum          : 12
			Min          : 12
			Max          : 12
			Attributes   :
				 ->  route: /users/:id
			Buckets
				 -inf to 0 : 0
				 0 to 5 : 0
				 5 to 10 : 0
				 10 to 25 : 1
				 25 to 50 : 0
				 50 to 75 : 0
				 75 to 100 : 0
				 100 to 250 : 0
				 250 to 500 : 0
				 500 to 750 : 0
				 750 to 1000 : 0
				 1000 to 2500 : 0
				 2500 to 5000 : 0
				 5000 to 7500 : 0
				 7500 to 10000 : 0
				10000 to +Infinity : 0
Metric #2
		Name         : requests
		Description  : 
		Unit         : 
		Type         : Sum
		Sum DataPoints
		Monotonic    : true
		Temporality  : Cumulative
		StartTime    : [..]
		EndTime      : [..]
		DataPoint #0
			Value        : 1
			Attributes   :
				 ->  route: /users/:id

```
