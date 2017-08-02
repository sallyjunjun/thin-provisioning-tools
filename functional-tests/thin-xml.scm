(library
  (thin-xml)
  (export generate-xml to-attribute-name)
  (import (rnrs)
          (list-utils)
          (fmt fmt)
          (only (srfi s1 lists) iota)
          (srfi s27 random-bits))

  ;;;----------------------------------------

  (define (make-const-generator n)
    (lambda () n))

  (define (make-uniform-generator low hi)
    (assert (<= low hi))

    (let ((range (- hi low)))
     (lambda ()
      (+ low (random-integer range)))))

  ;;;----------------------------------------

  (define (dquote doc)
    (cat (dsp #\") doc (dsp #\")))

  (define (to-attribute-name sym)
    (define (to-underscore c)
      (if (eq? #\- c) #\_ c))

    (list->string (map to-underscore (string->list (symbol->string sym)))))

  (define (attribute dotted-pair)
    (let ((key (to-attribute-name (car dotted-pair)))
          (val (cdr dotted-pair)))
      (cat (dsp key)
           (dsp "=")
           (dquote ((if (string? val) dsp wrt) val)))))

  (define (%open-tag sym attrs end)
    (cat (dsp "<")
         (dsp sym)
         (dsp " ")
         (apply cat (intersperse (dsp " ")
                                 (map attribute attrs)))
         (dsp end)))

  (define (open-tag sym attrs)
    (%open-tag sym attrs ">"))

  (define (simple-tag sym attrs)
    (%open-tag sym attrs "/>"))

  (define (close-tag sym)
    (cat (dsp "</")
         (dsp sym)
         (dsp ">")))

  (define (tag sym attrs . body)
    (if (null? body)
        (simple-tag sym attrs)
        (begin
          (cat (open-tag sym attrs)
               nl
               (apply cat body)
               nl
               (close-tag sym)))))

  (define (vcat docs)
    (apply cat (intersperse nl docs)))

  ;;;----------------------------------------

  (define (div-down n d)
    (floor (/ n d)))

  (define (generate-dev dev-id nr-mappings data-offset)
    (tag 'device `((dev-id . ,dev-id)
                   (mapped-blocks . ,nr-mappings)
                   (transaction . 1)
                   (creation-time . 0)
                   (snap-time . 0))
         (tag 'range_mapping `((origin-begin . 0)
                         (data-begin . ,data-offset)
                         (length . ,nr-mappings)
                         (time . 1)))))

  (define (generate-xml max-thins max-mappings)
    (let ((nr-thins ((make-uniform-generator 1 max-thins)))
          (nr-mappings-g (make-uniform-generator (div-down max-mappings 2)
                                                 max-mappings)))
      (let ((nr-mappings (iterate nr-mappings-g nr-thins)))
       (tag 'superblock `((uuid . "")
                         (time . 1)
                         (transaction . 1)
                         (flags . 0)
                         (version . 2)
                         (data-block-size . 128)
                         (nr-data-blocks . ,(apply + nr-mappings)))
            (vcat (map generate-dev
                       (iota nr-thins)
                       nr-mappings
                       (accumulate nr-mappings))))))))

